//! `sim --race [SPOT]`: many drivers reserve the same spot at the same moment.
//! With `INSERT ... IF NOT EXISTS`, exactly one of them may win.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use futures::future::join_all;
use reqwest::StatusCode;

use crate::api::Parkd;

pub async fn run(parkd: &Parkd, spot: &str, racers: usize) -> Result<()> {
    tracing::info!(%spot, racers, "firing concurrent reservations");

    let holders: Vec<String> = (0..racers).map(|i| format!("racer-{i:03}")).collect();
    let results = join_all(holders.iter().map(|holder| parkd.reserve(spot, holder))).await;

    let mut winners = Vec::new();
    let mut outcomes: BTreeMap<String, usize> = BTreeMap::new();
    for (holder, result) in holders.iter().zip(results) {
        let label = match result {
            Ok((status, _)) if status == StatusCode::CREATED => {
                winners.push(holder.clone());
                "201 reserved".to_owned()
            }
            Ok((status, body)) => format!("{} {body}", status.as_u16()),
            Err(err) => format!("request failed: {err}"),
        };
        *outcomes.entry(label).or_default() += 1;
    }

    for (outcome, count) in &outcomes {
        println!("{count:>4} × {outcome}");
    }

    match winners.as_slice() {
        [winner] => {
            println!("PASS: exactly one winner ({winner})");
            // Clean up so the spot is free for the next run.
            let status = parkd.cancel(spot, winner).await?;
            tracing::info!(%winner, %status, "cancelled the winning reservation");
            Ok(())
        }
        [] => bail!("no winner: is {spot} already taken, reserved or under maintenance?"),
        many => bail!("FAIL: {} winners for one spot: {many:?}", many.len()),
    }
}
