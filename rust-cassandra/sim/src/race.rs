//! `sim --race [SPOT]`: many drivers reserve the same spot at the same moment.
//! With `INSERT ... IF NOT EXISTS`, exactly one of them may win.
//!
//! `sim --race-without-lwt [SPOT]` runs the same race against a check-then-
//! insert reservation. Many drivers are told they won, but the row can only
//! hold one of them. Cancelling as each "winner" shows who really holds it,
//! because the cancel *is* an LWT (`IF reserved_by = ?`).

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use futures::future::join_all;
use reqwest::StatusCode;

use crate::api::Parkd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Lwt,
    WithoutLwt,
}

pub async fn run(parkd: &Parkd, spot: &str, racers: usize, mode: Mode) -> Result<()> {
    tracing::info!(%spot, racers, ?mode, "firing concurrent reservations");

    // Open one pooled connection per racer first. Otherwise the racer that
    // reuses the connection from the health check finishes before the others
    // have even connected, and there is no race at all.
    join_all((0..racers).map(|_| parkd.wait_until_up())).await;

    let holders: Vec<String> = (0..racers).map(|i| format!("racer-{i:03}")).collect();
    let results: Vec<Result<(StatusCode, String)>> = match mode {
        Mode::Lwt => join_all(holders.iter().map(|holder| parkd.reserve(spot, holder))).await,
        Mode::WithoutLwt => join_all(holders.iter().map(|holder| async move {
            let status = parkd.reserve_without_lwt(spot, holder).await?;
            Ok((status, String::new()))
        }))
        .await,
    };

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

    match mode {
        Mode::Lwt => match winners.as_slice() {
            [winner] => {
                println!("PASS: exactly one winner ({winner})");
                // Clean up so the spot is free for the next run.
                let status = parkd.cancel(spot, winner).await?;
                tracing::info!(%winner, %status, "cancelled the winning reservation");
                Ok(())
            }
            [] => bail!("no winner: is {spot} already taken, reserved or under maintenance?"),
            many => bail!("FAIL: {} winners for one spot: {many:?}", many.len()),
        },
        Mode::WithoutLwt => {
            if winners.is_empty() {
                bail!("no winner: is {spot} already reserved?");
            }
            // Only the real holder's cancel applies; the rest get 403. That
            // also cleans up the row.
            let cancels = join_all(winners.iter().map(|winner| parkd.cancel(spot, winner))).await;
            let real: Vec<&String> = winners
                .iter()
                .zip(&cancels)
                .filter(|(_, status)| matches!(status, Ok(StatusCode::NO_CONTENT)))
                .map(|(winner, _)| winner)
                .collect();
            println!(
                "{} drivers were told they won {spot}; the row holds {:?}. The other {} were lied to.",
                winners.len(),
                real,
                winners.len() - real.len()
            );
            Ok(())
        }
    }
}
