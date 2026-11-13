//! `active_session_by_plate` (open sessions, LWT writes only) and
//! `sessions_by_day` (closed sessions, for the daily report).

use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use futures::TryStreamExt;
use scylla::statement::Consistency;
use scylla::statement::unprepared::Statement;
use serde::Serialize;

use super::{Db, execute_lwt};

#[derive(Debug)]
pub enum Entry {
    /// A new session was opened.
    Opened,
    /// The same entry was already recorded: a duplicate camera read, or our
    /// own retried write after a contention timeout.
    Duplicate,
    /// The car already has a session that started at another time, so
    /// it never passed an exit camera. We keep the original session.
    AlreadyOpen { entered_at: DateTime<Utc> },
}

/// `IF NOT EXISTS` runs Paxos among the replicas: "is there a session?" and
/// "create it" happen as one step, so two camera reads can't both open one.
pub async fn open(db: &Db, plate: &str, entered_at: DateTime<Utc>, gate: &str) -> Result<Entry> {
    let lwt = execute_lwt(
        db,
        "INSERT INTO parkwatch.active_session_by_plate (plate, entered_at, entry_gate) VALUES (?, ?, ?) IF NOT EXISTS",
        (plate, entered_at, gate),
    )
    .await?;

    if lwt.applied {
        return Ok(Entry::Opened);
    }
    // CQL timestamps have millisecond precision.
    let existing = lwt.timestamp_millis("entered_at");
    Ok(match existing {
        Some(millis) if millis == entered_at.timestamp_millis() => Entry::Duplicate,
        Some(millis) => Entry::AlreadyOpen {
            entered_at: DateTime::from_timestamp_millis(millis).unwrap_or(entered_at),
        },
        None => Entry::Duplicate,
    })
}

#[derive(Debug)]
pub struct ActiveSession {
    pub entered_at: DateTime<Utc>,
}

/// Reads the open session at `SERIAL` consistency. A SERIAL read takes part in
/// Paxos, so it sees an LWT that is still in progress instead of a value from
/// before it. It costs more than a QUORUM read, which is fine at an exit gate.
pub async fn get_active(db: &Db, plate: &str) -> Result<Option<ActiveSession>> {
    let mut statement =
        Statement::new("SELECT entered_at FROM parkwatch.active_session_by_plate WHERE plate = ?");
    statement.set_consistency(Consistency::Serial);

    let row = db
        .execute_unpaged(statement, (plate,))
        .await?
        .into_rows_result()?
        .maybe_first_row::<(DateTime<Utc>,)>()?;
    Ok(row.map(|(entered_at,)| ActiveSession { entered_at }))
}

/// Closed sessions are keyed by `(day, exited_at, plate)`. A duplicate exit
/// event carries the same timestamp, so it rewrites the same row: no
/// double-counted visit and no double-charged fee.
pub async fn record_closed(
    db: &Db,
    plate: &str,
    entered_at: DateTime<Utc>,
    exited_at: DateTime<Utc>,
    duration_seconds: i64,
    fee_cents: i32,
) -> Result<()> {
    db.execute_unpaged(
        "INSERT INTO parkwatch.sessions_by_day (day, exited_at, plate, entered_at, duration_seconds, fee_cents) VALUES (?, ?, ?, ?, ?, ?)",
        (
            exited_at.date_naive(),
            exited_at,
            plate,
            entered_at,
            duration_seconds,
            fee_cents,
        ),
    )
    .await?;
    Ok(())
}

/// Deletes the open session, but only the one we billed: if the car has
/// re-entered meanwhile, `entered_at` no longer matches and nothing happens.
pub async fn close(db: &Db, plate: &str, entered_at: DateTime<Utc>) -> Result<bool> {
    let lwt = execute_lwt(
        db,
        "DELETE FROM parkwatch.active_session_by_plate WHERE plate = ? IF entered_at = ?",
        (plate, entered_at),
    )
    .await?;
    Ok(lwt.applied)
}

#[derive(Debug, Serialize)]
pub struct DailyReport {
    pub day: NaiveDate,
    pub visits: u64,
    pub average_stay_seconds: u64,
    pub revenue_cents: i64,
}

/// Reads one `sessions_by_day` partition. `execute_iter` fetches it page by
/// page (5000 rows per page by default), so a busy day never has to fit in a
/// single response.
pub async fn daily_report(db: &Db, day: NaiveDate) -> Result<DailyReport> {
    let mut rows = db
        .execute_iter(
            "SELECT duration_seconds, fee_cents FROM parkwatch.sessions_by_day WHERE day = ?",
            (day,),
        )
        .await?
        .rows_stream::<(Option<i64>, Option<i32>)>()?;

    let mut visits = 0_u64;
    let mut total_stay = 0_i64;
    let mut revenue_cents = 0_i64;
    while let Some((duration, fee)) = rows.try_next().await? {
        visits += 1;
        total_stay += duration.unwrap_or(0);
        revenue_cents += i64::from(fee.unwrap_or(0));
    }

    let average_stay_seconds = if visits == 0 {
        0
    } else {
        u64::try_from(total_stay).unwrap_or(0) / visits
    };
    Ok(DailyReport {
        day,
        visits,
        average_stay_seconds,
        revenue_cents,
    })
}
