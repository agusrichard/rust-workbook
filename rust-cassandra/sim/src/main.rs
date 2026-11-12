//! sim: deterministic spot-unit traffic for ParkWatch.
//!
//! `sim --seed` creates three levels of empty spots. With no arguments it
//! seeds and sends a small, repeatable event run including duplicate and late
//! messages, so the timestamp behaviour is easy to observe in Cassandra.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::{Duration as ChronoDuration, Utc};
use reqwest::StatusCode;
use serde::Serialize;
use tracing_subscriber::EnvFilter;

const LEVELS: [&str; 3] = ["B1", "B2", "B3"];
const SPOTS_PER_LEVEL: u16 = 40;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let seed_only = std::env::args()
        .skip(1)
        .any(|argument| argument == "--seed");
    let parkd_url = std::env::var("PARKD_URL").unwrap_or_else(|_| "http://parkd:8080".to_string());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let health = wait_for_parkd(&client, &parkd_url).await?;
    tracing::info!(%parkd_url, "parkd is up");
    tracing::debug!(%health, "parkd health response");

    seed(&client, &parkd_url).await?;
    if seed_only {
        tracing::info!("seed complete");
        return Ok(());
    }

    run_spot_events(&client, &parkd_url).await
}

/// Polls `GET /health` until parkd answers, since parkd itself waits for Cassandra.
async fn wait_for_parkd(client: &reqwest::Client, base: &str) -> Result<serde_json::Value> {
    const ATTEMPTS: u32 = 30;
    let url = format!("{base}/health");

    for attempt in 1..=ATTEMPTS {
        let result = async {
            client
                .get(&url)
                .send()
                .await?
                .error_for_status()?
                .json::<serde_json::Value>()
                .await
        }
        .await;

        match result {
            Ok(body) => return Ok(body),
            Err(err) if attempt < ATTEMPTS => {
                tracing::warn!(attempt, %err, "parkd not ready yet, retrying in 2s");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(err) => return Err(err).with_context(|| format!("parkd never answered at {url}")),
        }
    }
    unreachable!("the loop returns on the last attempt")
}

async fn seed(client: &reqwest::Client, base: &str) -> Result<()> {
    let response = client
        .post(format!("{base}/internal/seed"))
        .json(&serde_json::json!({ "spots_per_level": SPOTS_PER_LEVEL }))
        .send()
        .await?
        .error_for_status()?;
    let body: serde_json::Value = response.json().await?;
    tracing::info!(
        spots = body["spots"].as_u64().unwrap_or_default(),
        "spots seeded"
    );
    Ok(())
}

// Server-delivered overrides arrive with heartbeats in Phase 5. The simulator
// already owns the device-side rule so that behaviour does not change then.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum Override {
    Blue,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Light {
    Green,
    Red,
    Blue,
    Off,
}

impl Light {
    fn as_str(self) -> &'static str {
        match self {
            Self::Green => "green",
            Self::Red => "red",
            Self::Blue => "blue",
            Self::Off => "off",
        }
    }
}

/// The same rule a physical unit follows. Overrides do not turn a parked car
/// blue: a car is always red, while maintenance always turns the light off.
fn expected_light(occupied: bool, override_color: Option<Override>) -> Light {
    match (override_color, occupied) {
        (Some(Override::Off), _) => Light::Off,
        (_, true) => Light::Red,
        (Some(Override::Blue), false) => Light::Blue,
        (None, false) => Light::Green,
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum SpotEvent<'a> {
    Occupied {
        spot: &'a str,
        plate: &'a str,
        light: &'a str,
        ts: String,
    },
    Free {
        spot: &'a str,
        light: &'a str,
        ts: String,
    },
}

async fn post_event(client: &reqwest::Client, base: &str, event: &SpotEvent<'_>) -> Result<()> {
    let response = client
        .post(format!("{base}/events"))
        .json(event)
        .send()
        .await?;
    if response.status() != StatusCode::ACCEPTED {
        bail!(
            "parkd rejected event with {}: {}",
            response.status(),
            response.text().await?
        );
    }
    Ok(())
}

/// Sends 60 normal events, with exactly 10% duplicated and 10% delivered
/// late. A held-back occupied event arrives after its newer `free` event;
/// Cassandra must keep the later free state because its device timestamp is
/// greater.
async fn run_spot_events(client: &reqwest::Client, base: &str) -> Result<()> {
    let mut late_events = Vec::new();
    let start = Utc::now();

    for index in 0..30_u16 {
        let level = LEVELS[usize::from(index) % LEVELS.len()];
        let spot = format!("{level}-{:02}", (index % SPOTS_PER_LEVEL) + 1);
        let plate = format!("B{index:04}PW");
        let occupied_at = start + ChronoDuration::milliseconds(i64::from(index) * 10);
        let free_at = occupied_at + ChronoDuration::seconds(1);
        let occupied = SpotEvent::Occupied {
            spot: &spot,
            plate: &plate,
            light: expected_light(true, None).as_str(),
            ts: occupied_at.to_rfc3339(),
        };
        let free = SpotEvent::Free {
            spot: &spot,
            light: expected_light(false, None).as_str(),
            ts: free_at.to_rfc3339(),
        };

        if index % 5 == 0 {
            // Hold six of the 60 events. They become five seconds
            // late below, after a newer free event has already been ingested.
            late_events.push(serde_json::to_value(&occupied)?);
        } else {
            post_event(client, base, &occupied).await?;
        }
        post_event(client, base, &free).await?;

        // Six duplicates among the 60 normal device events.
        if index % 5 == 1 {
            post_event(client, base, &occupied).await?;
            tracing::info!(%spot, "sent duplicate spot event");
        }
    }

    tracing::info!(
        count = late_events.len(),
        "waiting five seconds before sending late events"
    );
    tokio::time::sleep(Duration::from_secs(5)).await;
    for event in late_events {
        let response = client
            .post(format!("{base}/events"))
            .json(&event)
            .send()
            .await?;
        if response.status() != StatusCode::ACCEPTED {
            bail!(
                "parkd rejected late event with {}: {}",
                response.status(),
                response.text().await?
            );
        }
        tracing::info!(event = %event, "sent late spot event; Cassandra should retain the newer free state");
    }
    tracing::info!("simulation complete: all exercised spots should be free and green");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Light, Override, expected_light};

    #[test]
    fn physical_light_rule_order_is_preserved() {
        assert_eq!(expected_light(false, None), Light::Green);
        assert_eq!(expected_light(false, Some(Override::Blue)), Light::Blue);
        assert_eq!(expected_light(true, Some(Override::Blue)), Light::Red);
        assert_eq!(expected_light(true, Some(Override::Off)), Light::Off);
    }
}
