//! `sensor_heartbeats`: one row per unit that has been heard from recently.
//!
//! Each heartbeat rewrites the row with a 60-second TTL. A unit that goes
//! silent for 60 seconds has no row left, so "no row" means "offline", and
//! nothing ever has to scan for stale timestamps.

use std::collections::HashSet;

use anyhow::Result;
use chrono::{DateTime, Utc};

use super::Db;
use crate::lights::Light;

/// Four missed heartbeats (sent every 15s) and the unit counts as offline.
pub const HEARTBEAT_TTL_SECONDS: i32 = 60;

pub async fn record(
    db: &Db,
    level: &str,
    spot_id: &str,
    battery: u8,
    light: Light,
    seen_at: DateTime<Utc>,
) -> Result<()> {
    // TTL expiry is counted from when the replica stores the write, not from
    // the device timestamp, so a unit with a skewed clock still gets 60s.
    db.execute_unpaged(
        "INSERT INTO parkwatch.sensor_heartbeats (level, spot_id, battery, light, last_seen) VALUES (?, ?, ?, ?, ?) USING TIMESTAMP ? AND TTL ?",
        (
            level,
            spot_id,
            i32::from(battery),
            light.as_str(),
            seen_at,
            seen_at.timestamp_micros(),
            HEARTBEAT_TTL_SECONDS,
        ),
    )
    .await?;
    Ok(())
}

/// The spot ids on one level that sent a heartbeat in the last 60 seconds.
pub async fn online_spots(db: &Db, level: &str) -> Result<HashSet<String>> {
    let rows = db
        .execute_unpaged(
            "SELECT spot_id FROM parkwatch.sensor_heartbeats WHERE level = ?",
            (level,),
        )
        .await?
        .into_rows_result()?;

    let mut online = HashSet::new();
    for row in rows.rows::<(String,)>()? {
        online.insert(row?.0);
    }
    Ok(online)
}
