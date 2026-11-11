//! sim: simulated spot units and gate cameras.
//!
//! Phase 0 only checks that sim can reach parkd over the compose network.
//! The device simulation arrives in Phase 2.

use std::time::Duration;

use anyhow::{Context, Result};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let parkd_url = std::env::var("PARKD_URL").unwrap_or_else(|_| "http://parkd:8080".to_string());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let health = wait_for_parkd(&client, &parkd_url).await?;
    tracing::info!(%parkd_url, "parkd is up");
    println!("{}", serde_json::to_string_pretty(&health)?);
    Ok(())
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
