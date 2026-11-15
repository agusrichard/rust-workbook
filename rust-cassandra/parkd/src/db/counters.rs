//! `free_count_by_level`: a counter of unoccupied spots per level (Phase 7).
//!
//! Every other device write in parkd is idempotent: replaying an event
//! rewrites the same cell with the same timestamp. A counter update is not.
//! `free = free - 1` applied twice subtracts 2. So the counter drifts from the
//! real state whenever an event is applied more than once:
//!
//! - a unit sends the same event twice (the sim does this for 10% of events)
//! - parkd returns 503 after the counter write actually landed, and the unit
//!   retries the whole event
//! - a coordinator dies after forwarding the write, the write still lands on
//!   the replicas, and the client sees an error
//!
//! Cassandra can't tell a retry from a new increment, and the driver refuses
//! to retry counter writes for the same reason. Compare the counter with the
//! real state in `GET /ops/counters`.

use anyhow::Result;

use super::{Db, relaxed};

/// Adds `delta` (negative to subtract). Counter writes can't use
/// `USING TIMESTAMP` or TTL: the value is a sum, not a cell that a newer write
/// replaces.
pub async fn add_free(db: &Db, level: &str, delta: i64) -> Result<()> {
    db.execute_unpaged(
        relaxed("UPDATE parkwatch.free_count_by_level SET free = free + ? WHERE level = ?"),
        (scylla::value::Counter(delta), level),
    )
    .await?;
    Ok(())
}

pub async fn free(db: &Db, level: &str) -> Result<i64> {
    let row = db
        .execute_unpaged(
            relaxed("SELECT free FROM parkwatch.free_count_by_level WHERE level = ?"),
            (level,),
        )
        .await?
        .into_rows_result()?
        .maybe_first_row::<(Option<scylla::value::Counter>,)>()?;
    Ok(row.and_then(|(counter,)| counter).map_or(0, |counter| counter.0))
}

/// A counter can't be set, only incremented. To reset it, read it and add the
/// difference. If anything updates it in between, the reset is off by that
/// much, which is fine for `sim --seed` on a quiet car park.
pub async fn reset_free(db: &Db, level: &str, value: i64) -> Result<()> {
    let current = free(db, level).await?;
    if current != value {
        add_free(db, level, value - current).await?;
    }
    Ok(())
}
