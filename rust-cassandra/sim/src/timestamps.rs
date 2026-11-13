//! `sim --timestamps`: the Phase 2 check that `USING TIMESTAMP` makes
//! duplicate and late device events harmless.

use std::time::Duration;

use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};

use crate::api::{Event, Parkd};
use crate::lights::expected_light;
use crate::{LEVELS, SPOTS_PER_LEVEL};

/// Sends 60 normal events, with exactly 10% duplicated and 10% delivered
/// late. A held-back occupied event arrives after its newer `free` event;
/// Cassandra must keep the later free state because its device timestamp is
/// greater.
pub async fn run(parkd: &Parkd) -> Result<()> {
    let mut late_events = Vec::new();
    let start = Utc::now();

    for index in 0..30_u16 {
        let level = LEVELS[usize::from(index) % LEVELS.len()];
        let spot = format!("{level}-{:02}", (index % SPOTS_PER_LEVEL) + 1);
        let plate = format!("B{index:04}PW");
        let occupied_at = start + ChronoDuration::milliseconds(i64::from(index) * 10);
        let free_at = occupied_at + ChronoDuration::seconds(1);
        let occupied = Event::Occupied {
            spot: spot.clone(),
            plate,
            light: expected_light(true, None),
            ts: occupied_at,
        };
        let free = Event::Free {
            spot: spot.clone(),
            light: expected_light(false, None),
            ts: free_at,
        };

        if index % 5 == 0 {
            // Hold six of the 60 events. They become five seconds
            // late below, after a newer free event has already been ingested.
            late_events.push(occupied.clone());
        } else {
            parkd.send(&occupied).await?;
        }
        parkd.send(&free).await?;

        // Six duplicates among the 60 normal device events.
        if index % 5 == 1 {
            parkd.send(&occupied).await?;
            tracing::info!(%spot, "sent duplicate spot event");
        }
    }

    tracing::info!(
        count = late_events.len(),
        "waiting five seconds before sending late events"
    );
    tokio::time::sleep(Duration::from_secs(5)).await;
    for event in late_events {
        parkd.send(&event).await?;
        tracing::info!(
            ?event,
            "sent late spot event; Cassandra should retain the newer free state"
        );
    }
    tracing::info!("simulation complete: all exercised spots should be free and green");
    Ok(())
}
