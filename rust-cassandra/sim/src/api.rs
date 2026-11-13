//! A thin HTTP client for parkd, plus the device event wire format.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::lights::{Light, Override};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Event {
    Occupied {
        spot: String,
        plate: String,
        light: Light,
        ts: DateTime<Utc>,
    },
    Free {
        spot: String,
        light: Light,
        ts: DateTime<Utc>,
    },
    Heartbeat {
        spot: String,
        light: Light,
        battery: u8,
        ts: DateTime<Utc>,
    },
    Gate {
        gate: String,
        direction: Direction,
        plate: String,
        ts: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    In,
    Out,
}

#[derive(Debug, Deserialize)]
struct HeartbeatReply {
    #[serde(rename = "override")]
    override_color: Option<Override>,
}

pub struct Parkd {
    client: reqwest::Client,
    base: String,
}

impl Parkd {
    pub fn new(base: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self { client, base })
    }

    /// Polls `GET /health` until parkd answers, since parkd itself waits for Cassandra.
    pub async fn wait_until_up(&self) -> Result<Value> {
        const ATTEMPTS: u32 = 30;
        let url = format!("{}/health", self.base);

        for attempt in 1..=ATTEMPTS {
            let result = async {
                self.client
                    .get(&url)
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await
            }
            .await;

            match result {
                Ok(body) => return Ok(body),
                Err(err) if attempt < ATTEMPTS => {
                    tracing::warn!(attempt, %err, "parkd not ready yet, retrying in 2s");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                Err(err) => {
                    return Err(err).with_context(|| format!("parkd never answered at {url}"));
                }
            }
        }
        unreachable!("the loop returns on the last attempt")
    }

    pub async fn seed(&self, spots_per_level: u16) -> Result<u64> {
        let body: Value = self
            .client
            .post(format!("{}/internal/seed", self.base))
            .json(&serde_json::json!({ "spots_per_level": spots_per_level }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(body["spots"].as_u64().unwrap_or_default())
    }

    /// Posts one device event and returns parkd's JSON reply.
    pub async fn send(&self, event: &Event) -> Result<Value> {
        let response = self
            .client
            .post(format!("{}/events", self.base))
            .json(event)
            .send()
            .await?;
        let status = response.status();
        if status != StatusCode::ACCEPTED {
            bail!("parkd rejected event with {status}: {}", response.text().await?);
        }
        Ok(response.json().await?)
    }

    /// Like `send`, but keeps trying for about 10 seconds, the way a real unit
    /// would buffer an event while the server is unreachable. Resending is safe
    /// because parkd's writes are idempotent: spot events carry the device
    /// timestamp, and gate reads are deduplicated by the LWT.
    pub async fn send_with_retry(&self, event: &Event) -> Result<Value> {
        const ATTEMPTS: u32 = 5;
        for attempt in 1..=ATTEMPTS {
            match self.send(event).await {
                Ok(reply) => return Ok(reply),
                Err(err) if attempt < ATTEMPTS => {
                    tracing::debug!(attempt, error = format!("{err:#}"), "send failed, retrying in 2s");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                Err(err) => return Err(err),
            }
        }
        unreachable!("the loop returns on the last attempt")
    }

    /// Sends a heartbeat and returns the override the server wants.
    pub async fn heartbeat(&self, event: &Event) -> Result<Option<Override>> {
        let reply: HeartbeatReply = serde_json::from_value(self.send(event).await?)?;
        Ok(reply.override_color)
    }

    /// Returns the status and body as-is, because 409 is an expected answer.
    pub async fn reserve(&self, spot: &str, reserved_by: &str) -> Result<(StatusCode, String)> {
        let response = self
            .client
            .post(format!("{}/spots/{spot}/reserve", self.base))
            .json(&serde_json::json!({ "reserved_by": reserved_by }))
            .send()
            .await?;
        Ok((response.status(), response.text().await?))
    }

    pub async fn cancel(&self, spot: &str, reserved_by: &str) -> Result<StatusCode> {
        let response = self
            .client
            .delete(format!("{}/spots/{spot}/reserve", self.base))
            .query(&[("reserved_by", reserved_by)])
            .send()
            .await?;
        Ok(response.status())
    }
}
