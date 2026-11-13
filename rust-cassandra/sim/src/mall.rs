//! `sim` with no flags: a mall that runs until stopped.
//!
//! - Every spot unit heartbeats every 15s and obeys the override in the reply.
//! - Cars arrive at random, pass the entry camera, sometimes reserve a spot
//!   first, park, stay a while, leave the spot, and pass the exit camera.
//! - On purpose: 10% of events are sent twice, 10% of spot events arrive 5s
//!   late, a few units stop heartbeating, and a few bulbs are broken.

use std::ops::Range;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Utc;
use rand::seq::SliceRandom;
use reqwest::StatusCode;

use crate::LEVELS;
use crate::api::{Direction, Event, Parkd};
use crate::lights::{Light, Override, expected_light};

const HEARTBEAT_EVERY: Duration = Duration::from_secs(15);
const ARRIVAL_EVERY_MS: Range<u64> = 1_000..3_000;
const STAY_SECONDS: Range<u64> = 30..180;
const DRIVE_SECONDS: Range<u64> = 3..8;
const DUPLICATE_RATE: f64 = 0.10;
const LATE_RATE: f64 = 0.10;
const LATE_BY: Duration = Duration::from_secs(5);
const RESERVE_RATE: f64 = 0.15;
const BROKEN_BULBS: usize = 2;
const SILENT_UNITS: usize = 3;
const SILENT_AFTER: Duration = Duration::from_secs(45);

struct Unit {
    spot: String,
    occupant: Option<String>,
    override_color: Option<Override>,
    broken_bulb: bool,
    battery: u8,
}

impl Unit {
    /// What the light physically shows. The unit drives the bulb by the rule,
    /// but a broken bulb stays dark, and its light sensor reports that.
    fn light(&self) -> Light {
        if self.broken_bulb {
            Light::Off
        } else {
            expected_light(self.occupant.is_some(), self.override_color)
        }
    }

    /// A driver only picks a spot whose light says it's free.
    fn looks_free(&self) -> bool {
        self.occupant.is_none() && self.override_color.is_none()
    }
}

type Units = Arc<Mutex<Vec<Unit>>>;

fn sleep_ms(range: Range<u64>) -> tokio::time::Sleep {
    tokio::time::sleep(Duration::from_millis(rand::random_range(range)))
}

fn sleep_s(range: Range<u64>) -> tokio::time::Sleep {
    tokio::time::sleep(Duration::from_secs(rand::random_range(range)))
}

pub async fn run(parkd: Arc<Parkd>, spots_per_level: u16) -> Result<()> {
    let mut units: Vec<Unit> = LEVELS
        .iter()
        .flat_map(|level| (1..=spots_per_level).map(move |n| format!("{level}-{n:02}")))
        .map(|spot| Unit {
            spot,
            occupant: None,
            override_color: None,
            broken_bulb: false,
            battery: rand::random_range(60..=100),
        })
        .collect();

    // Pick distinct units for the planted faults.
    let mut picks: Vec<usize> = (0..units.len()).collect();
    picks.shuffle(&mut rand::rng());
    let (broken, rest) = picks.split_at(BROKEN_BULBS);
    let silent = &rest[..SILENT_UNITS];
    for &index in broken {
        units[index].broken_bulb = true;
    }
    let broken_spots: Vec<&str> = broken.iter().map(|&i| units[i].spot.as_str()).collect();
    let silent_spots: Vec<&str> = silent.iter().map(|&i| units[i].spot.as_str()).collect();
    tracing::info!(spots = ?broken_spots, "planted broken bulbs: expect them in GET /ops/faults");
    tracing::info!(spots = ?silent_spots, after = ?SILENT_AFTER, "these units will stop heartbeating: expect them in GET /ops/offline about 60s later");

    let count = units.len();
    let units: Units = Arc::new(Mutex::new(units));

    for index in 0..count {
        let stops_after = silent.contains(&index).then_some(SILENT_AFTER);
        tokio::spawn(heartbeat_loop(
            parkd.clone(),
            units.clone(),
            index,
            stops_after,
        ));
    }
    tokio::spawn(report_loop(units.clone()));

    loop {
        sleep_ms(ARRIVAL_EVERY_MS).await;
        let (parkd, units) = (parkd.clone(), units.clone());
        tokio::spawn(async move {
            if let Err(err) = car(&parkd, &units).await {
                tracing::warn!(error = format!("{err:#}"), "car journey failed");
            }
        });
    }
}

/// Every 15s: tell the server we're alive and what the light shows, and apply
/// whatever override comes back.
async fn heartbeat_loop(
    parkd: Arc<Parkd>,
    units: Units,
    index: usize,
    stops_after: Option<Duration>,
) {
    let started = Instant::now();
    // Spread the 120 units over the interval instead of all at once.
    sleep_ms(0..HEARTBEAT_EVERY.as_millis() as u64).await;

    loop {
        let event = {
            let mut units = units.lock().unwrap();
            let unit = &mut units[index];
            if let Some(after) = stops_after
                && started.elapsed() >= after
            {
                tracing::warn!(spot = %unit.spot, "heartbeats stopped (simulated outage)");
                return;
            }
            if rand::random_bool(0.05) {
                unit.battery = unit.battery.saturating_sub(1).max(5);
            }
            Event::Heartbeat {
                spot: unit.spot.clone(),
                light: unit.light(),
                battery: unit.battery,
                ts: Utc::now(),
            }
        };

        match parkd.heartbeat(&event).await {
            Ok(wanted) => {
                let mut units = units.lock().unwrap();
                let unit = &mut units[index];
                if unit.override_color != wanted {
                    let before = unit.light();
                    unit.override_color = wanted;
                    tracing::info!(
                        spot = %unit.spot,
                        from = ?before,
                        to = ?unit.light(),
                        override_color = ?wanted,
                        "light changed by server override"
                    );
                }
            }
            Err(err) => tracing::warn!(error = format!("{err:#}"), "heartbeat failed"),
        }
        tokio::time::sleep(HEARTBEAT_EVERY).await;
    }
}

async fn report_loop(units: Units) {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let units = units.lock().unwrap();
        let occupied = units.iter().filter(|u| u.occupant.is_some()).count();
        let reserved = units
            .iter()
            .filter(|u| u.override_color == Some(Override::Blue))
            .count();
        let maintenance = units
            .iter()
            .filter(|u| u.override_color == Some(Override::Off))
            .count();
        tracing::info!(
            occupied,
            reserved,
            maintenance,
            total = units.len(),
            "mall status"
        );
    }
}

fn random_plate() -> String {
    let letter = || char::from(b'A' + rand::random_range(0..26_u8));
    format!(
        "B{:04}{}{}",
        rand::random_range(1..10_000),
        letter(),
        letter()
    )
}

/// One car's visit, start to finish.
async fn car(parkd: &Arc<Parkd>, units: &Units) -> Result<()> {
    let plate = random_plate();
    send_gate(parkd, "entry-1", Direction::In, &plate).await?;

    let reserved = if rand::random_bool(RESERVE_RATE) {
        reserve_spot(parkd, units, &plate).await?
    } else {
        None
    };

    sleep_s(DRIVE_SECONDS).await;
    let Some(index) = park(parkd, units, &plate, reserved).await else {
        tracing::info!(%plate, "no free spot found, leaving");
        sleep_s(DRIVE_SECONDS).await;
        send_gate(parkd, "exit-1", Direction::Out, &plate).await?;
        return Ok(());
    };

    sleep_s(STAY_SECONDS).await;
    let free = {
        let mut units = units.lock().unwrap();
        let unit = &mut units[index];
        unit.occupant = None;
        Event::Free {
            spot: unit.spot.clone(),
            light: unit.light(),
            ts: Utc::now(),
        }
    };
    deliver(parkd, free);

    sleep_s(DRIVE_SECONDS).await;
    send_gate(parkd, "exit-1", Direction::Out, &plate).await
}

/// Reserves a random spot that looks free. A 409 means another driver got it
/// first, which is the LWT doing its job.
async fn reserve_spot(parkd: &Parkd, units: &Units, plate: &str) -> Result<Option<usize>> {
    let candidate = {
        let units = units.lock().unwrap();
        let free: Vec<usize> = (0..units.len()).filter(|&i| units[i].looks_free()).collect();
        free.get(rand::random_range(0..free.len().max(1)))
            .map(|&i| (i, units[i].spot.clone()))
    };
    let Some((index, spot)) = candidate else {
        return Ok(None);
    };

    let (status, body) = parkd.reserve(&spot, plate).await?;
    if status == StatusCode::CREATED {
        tracing::info!(%plate, %spot, "driver reserved a spot");
        Ok(Some(index))
    } else {
        tracing::info!(%plate, %spot, %status, %body, "reservation refused");
        Ok(None)
    }
}

/// Finds a spot (the reserved one if still usable), turns its light red and
/// reports `occupied`. Tries a few times if the car park is full.
async fn park(
    parkd: &Arc<Parkd>,
    units: &Units,
    plate: &str,
    reserved: Option<usize>,
) -> Option<usize> {
    for _ in 0..3 {
        let parked = {
            let mut units = units.lock().unwrap();
            let usable_reservation = reserved.filter(|&i| {
                units[i].occupant.is_none() && units[i].override_color != Some(Override::Off)
            });
            let choice = usable_reservation.or_else(|| {
                let free: Vec<usize> =
                    (0..units.len()).filter(|&i| units[i].looks_free()).collect();
                free.get(rand::random_range(0..free.len().max(1))).copied()
            });
            choice.map(|index| {
                let unit = &mut units[index];
                unit.occupant = Some(plate.to_owned());
                let event = Event::Occupied {
                    spot: unit.spot.clone(),
                    plate: plate.to_owned(),
                    light: unit.light(),
                    ts: Utc::now(),
                };
                (index, event)
            })
        };

        if let Some((index, event)) = parked {
            deliver(parkd, event);
            return Some(index);
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    None
}

/// Sends a spot event like a flaky network would: usually once, sometimes
/// twice, sometimes 5 seconds late. Either way the event keeps the timestamp
/// from when the sensor saw the change.
fn deliver(parkd: &Arc<Parkd>, event: Event) {
    let parkd = parkd.clone();
    tokio::spawn(async move {
        if rand::random_bool(LATE_RATE) {
            tokio::time::sleep(LATE_BY).await;
            tracing::info!(?event, "delivering late event");
        }
        if let Err(err) = parkd.send_with_retry(&event).await {
            tracing::warn!(error = format!("{err:#}"), "spot event lost");
            return;
        }
        if rand::random_bool(DUPLICATE_RATE) {
            let _ = parkd.send(&event).await;
        }
    });
}

/// Gate reads are never late (see `gate_in` in parkd), but a camera can read
/// the same plate twice.
async fn send_gate(parkd: &Parkd, gate: &str, direction: Direction, plate: &str) -> Result<()> {
    let event = Event::Gate {
        gate: gate.to_owned(),
        direction,
        plate: plate.to_owned(),
        ts: Utc::now(),
    };
    let reply = parkd.send_with_retry(&event).await?;
    if matches!(direction, Direction::Out) && reply["session"] == "closed" {
        tracing::info!(
            %plate,
            stay_seconds = reply["duration_seconds"].as_i64(),
            fee_cents = reply["fee_cents"].as_i64(),
            "car exited"
        );
    }
    if rand::random_bool(DUPLICATE_RATE) {
        let duplicate = parkd.send(&event).await?;
        tracing::debug!(%plate, %gate, reply = %duplicate, "duplicate gate read");
    }
    Ok(())
}
