//! `reservations`, written only with lightweight transactions.
//!
//! Mixing plain writes with LWT writes on the same rows breaks the guarantee:
//! a plain write skips Paxos and can land in the middle of a Paxos round. So
//! every statement here, including deletes, has an `IF` clause.

use anyhow::Result;
use chrono::{DateTime, Utc};

use super::{Db, execute_lwt};

#[derive(Debug)]
pub enum Reserve {
    Reserved,
    /// The holder is already this driver: a double-click, or our own retried
    /// write after a contention timeout.
    AlreadyYours,
    /// Someone else got it first.
    Taken,
}

/// `IF NOT EXISTS ... USING TTL`: exactly one concurrent caller wins, and the
/// row deletes itself after `ttl_seconds` with no cleanup job.
pub async fn reserve(
    db: &Db,
    spot_id: &str,
    reserved_by: &str,
    reserved_at: DateTime<Utc>,
    ttl_seconds: i32,
) -> Result<Reserve> {
    let lwt = execute_lwt(
        db,
        "INSERT INTO parkwatch.reservations (spot_id, reserved_by, reserved_at) VALUES (?, ?, ?) IF NOT EXISTS USING TTL ?",
        (spot_id, reserved_by, reserved_at, ttl_seconds),
    )
    .await?;

    Ok(if lwt.applied {
        Reserve::Reserved
    } else if lwt.text("reserved_by") == Some(reserved_by) {
        Reserve::AlreadyYours
    } else {
        Reserve::Taken
    })
}

#[derive(Debug)]
pub enum Cancel {
    Cancelled,
    NotHolder,
    NotFound,
}

/// Only the holder can cancel: the delete applies only if `reserved_by` matches.
pub async fn cancel(db: &Db, spot_id: &str, reserved_by: &str) -> Result<Cancel> {
    let lwt = execute_lwt(
        db,
        "DELETE FROM parkwatch.reservations WHERE spot_id = ? IF reserved_by = ?",
        (spot_id, reserved_by),
    )
    .await?;

    // When the row doesn't exist, `[applied] = false` comes back with
    // `reserved_by = null`.
    Ok(if lwt.applied {
        Cancel::Cancelled
    } else if lwt.text("reserved_by").is_some() {
        Cancel::NotHolder
    } else {
        Cancel::NotFound
    })
}

/// A car parked in the spot, so the reservation is used up.
///
/// The condition is `reserved_at < arrival` rather than plain `IF EXISTS`:
/// a late `occupied` event from a car that parked *before* the reservation
/// was made must not cancel it. This compares the device clock (arrival) with
/// the server clock (`reserved_at`), so a unit whose clock runs far behind
/// would leave the reservation to expire by TTL instead. Since an occupied
/// spot shows red either way, that is harmless.
pub async fn claim_on_arrival(db: &Db, spot_id: &str, arrived_at: DateTime<Utc>) -> Result<bool> {
    let lwt = execute_lwt(
        db,
        "DELETE FROM parkwatch.reservations WHERE spot_id = ? IF reserved_at < ?",
        (spot_id, arrived_at),
    )
    .await?;
    Ok(lwt.applied)
}

/// Maintenance cancels any reservation on the spot.
pub async fn drop_any(db: &Db, spot_id: &str) -> Result<bool> {
    let lwt = execute_lwt(
        db,
        "DELETE FROM parkwatch.reservations WHERE spot_id = ? IF EXISTS",
        (spot_id,),
    )
    .await?;
    Ok(lwt.applied)
}
