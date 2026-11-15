//! sim: simulated spot units and gate cameras for ParkWatch.
//!
//! ```text
//! sim                  seed, then run the mall until stopped
//! sim --seed           create 3 levels x 40 empty spots, then exit
//! sim --timestamps     seed, then send duplicate and late spot events (Phase 2)
//! sim --race [SPOT]    100 concurrent reservations for one spot (Phase 4)
//! sim --race-without-lwt [SPOT]
//!                      the same race with check-then-insert instead of LWT
//! ```
//!
//! The mall reads `SIM_SPEED`, `SIM_DUPLICATE_RATE` and `SIM_LATE_RATE`; see
//! `mall.rs`.

mod api;
mod lights;
mod mall;
mod race;
mod timestamps;

use std::sync::Arc;

use anyhow::{Result, bail};
use tracing_subscriber::EnvFilter;

use crate::api::Parkd;

pub const LEVELS: [&str; 3] = ["B1", "B2", "B3"];
pub const SPOTS_PER_LEVEL: u16 = 40;

const RACERS: usize = 100;
const DEFAULT_RACE_SPOT: &str = "B3-40";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let parkd_url = std::env::var("PARKD_URL").unwrap_or_else(|_| "http://parkd:8080".to_string());
    let parkd = Arc::new(Parkd::new(parkd_url.clone())?);

    let health = parkd.wait_until_up().await?;
    tracing::info!(%parkd_url, "parkd is up");
    tracing::debug!(%health, "parkd health response");

    match args.first().map(String::as_str) {
        // The race doesn't reseed: resetting every spot mid-run would fight
        // with a mall simulation that may be running next to it.
        Some(flag @ ("--race" | "--race-without-lwt")) => {
            let spot = args.get(1).map_or(DEFAULT_RACE_SPOT, String::as_str);
            let mode = if flag == "--race" {
                race::Mode::Lwt
            } else {
                race::Mode::WithoutLwt
            };
            race::run(&parkd, spot, RACERS, mode).await
        }
        Some("--seed") => seed(&parkd).await,
        Some("--timestamps") => {
            seed(&parkd).await?;
            timestamps::run(&parkd).await
        }
        None => {
            seed(&parkd).await?;
            mall::run(parkd, SPOTS_PER_LEVEL).await
        }
        Some(other) => bail!("unknown argument {other:?}; see the top of sim/src/main.rs"),
    }
}

async fn seed(parkd: &Parkd) -> Result<()> {
    let spots = parkd.seed(SPOTS_PER_LEVEL).await?;
    tracing::info!(spots, "spots seeded");
    Ok(())
}
