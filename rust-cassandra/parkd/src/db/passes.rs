//! `passes_by_plate`: monthly passes, checked at the exit gate (Phase 7).
//!
//! A pass row carries a TTL that ends when the pass does, so expired passes
//! disappear without a cleanup job. `valid_until` is still stored and checked,
//! because the exit gate compares it with the *camera's* exit time, which can
//! be a little behind the server's clock.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::{Db, strict};

#[derive(Debug, Clone, Serialize)]
pub struct Pass {
    pub plate: String,
    pub holder: String,
    pub valid_until: DateTime<Utc>,
}

impl Pass {
    pub fn covers(&self, at: DateTime<Utc>) -> bool {
        at <= self.valid_until
    }
}

/// Buying again replaces the old pass. A plain write is fine here: the
/// operator's last word wins, and there is no "only one may succeed" rule.
pub async fn put(db: &Db, pass: &Pass, ttl_seconds: i32) -> Result<()> {
    db.execute_unpaged(
        strict("INSERT INTO parkwatch.passes_by_plate (plate, holder, valid_until) VALUES (?, ?, ?) USING TTL ?"),
        (&pass.plate, &pass.holder, pass.valid_until, ttl_seconds),
    )
    .await?;
    Ok(())
}

pub async fn get(db: &Db, plate: &str) -> Result<Option<Pass>> {
    let row = db
        .execute_unpaged(
            strict("SELECT holder, valid_until FROM parkwatch.passes_by_plate WHERE plate = ?"),
            (plate,),
        )
        .await?
        .into_rows_result()?
        .maybe_first_row::<(String, DateTime<Utc>)>()?;
    Ok(row.map(|(holder, valid_until)| Pass {
        plate: plate.to_owned(),
        holder,
        valid_until,
    }))
}

pub async fn delete(db: &Db, plate: &str) -> Result<()> {
    db.execute_unpaged(
        strict("DELETE FROM parkwatch.passes_by_plate WHERE plate = ?"),
        (plate,),
    )
    .await?;
    Ok(())
}
