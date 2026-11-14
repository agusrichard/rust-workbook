//! `POST /events`: everything spot units and gate cameras send.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::AppState;
use crate::db::{counters, heartbeats, passes, reservations, sessions, spots};
use crate::error::{ApiResult, internal, split_spot_id};
use crate::fees::fee_cents;
use crate::lights::{Light, Override};

/// The wire format sent by a spot unit or gate camera. Keeping it as a tagged
/// enum prevents impossible combinations such as a `free` event with a plate.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DeviceEvent {
    Occupied {
        spot: String,
        plate: String,
        light: Light,
        ts: DateTime<Utc>,
    },
    Free {
        spot: String,
        light: Light,
        ts: DateTime<Utc>,
    },
    Heartbeat {
        spot: String,
        light: Light,
        battery: u8,
        ts: DateTime<Utc>,
    },
    Gate {
        gate: String,
        direction: Direction,
        plate: String,
        ts: DateTime<Utc>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    In,
    Out,
}

/// Every event gets `202 Accepted` and a small JSON body. The heartbeat body is
/// the one devices act on: `{ "override": null | "blue" | "off" }`.
pub async fn ingest(
    State(state): State<AppState>,
    Json(event): Json<DeviceEvent>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let body = match event {
        DeviceEvent::Occupied {
            spot,
            plate,
            light,
            ts,
        } => occupied(&state, &spot, &plate, light, ts).await?,
        DeviceEvent::Free { spot, light, ts } => free(&state, &spot, light, ts).await?,
        DeviceEvent::Heartbeat {
            spot,
            light,
            battery,
            ts,
        } => heartbeat(&state, &spot, light, battery, ts).await?,
        DeviceEvent::Gate {
            gate,
            direction: Direction::In,
            plate,
            ts,
        } => gate_in(&state, &gate, &plate, ts).await?,
        DeviceEvent::Gate {
            gate,
            direction: Direction::Out,
            plate,
            ts,
        } => gate_out(&state, &gate, &plate, ts).await?,
    };
    Ok((StatusCode::ACCEPTED, Json(body)))
}

/// CAR PARKS: spot state, "find my car", history and the counter are
/// independent writes, so they run concurrently. All of them run at `ONE`,
/// so a car parking is recorded even with 2 of 3 nodes down.
async fn occupied(
    state: &AppState,
    spot: &str,
    plate: &str,
    light: Light,
    ts: DateTime<Utc>,
) -> ApiResult<Value> {
    let (level, spot_id) = split_spot_id(spot)?;
    let db = &state.db;

    tokio::try_join!(
        spots::update_spot_state(
            db,
            level,
            spot_id,
            true,
            Some(plate),
            light,
            ts.timestamp_micros()
        ),
        spots::set_car_location(db, plate, level, spot_id, ts),
        spots::record_spot_event(db, spot_id, ts, "occupied", Some(plate), light),
        counters::add_free(db, level, -1),
        zone_write(state, level, spot_id, Some((true, Some(plate))), light, ts),
    )
    .map_err(internal)?;

    let reservation_claimed = claim_reservation(state, level, spot_id, plate, ts).await;
    Ok(json!({ "stored": "occupied", "reservation_claimed": reservation_claimed }))
}

/// If the spot was reserved, the reservation is used up now. Clear the blue
/// override only if our delete was the one that applied.
///
/// This is best-effort. The claim is an LWT, which needs 2 of 3 nodes, and
/// the occupancy written above must not depend on that. If the claim fails,
/// the reservation and its blue override expire by TTL. The light shows red
/// meanwhile anyway, because a car is in the spot. `None` means "unknown".
async fn claim_reservation(
    state: &AppState,
    level: &str,
    spot_id: &str,
    plate: &str,
    ts: DateTime<Utc>,
) -> Option<bool> {
    let db = &state.db;
    match reservations::claim_on_arrival(db, spot_id, ts).await {
        Ok(true) => {
            if let Err(err) = spots::clear_override_if(db, level, spot_id, Override::Blue).await {
                tracing::warn!(%spot_id, error = format!("{err:#}"), "reservation claimed but blue override not cleared; it will expire by TTL");
            }
            tracing::info!(%spot_id, %plate, "reserved spot taken, reservation cleared");
            Some(true)
        }
        Ok(false) => Some(false),
        Err(err) => {
            tracing::warn!(%spot_id, error = format!("{err:#}"), "could not check the reservation (no quorum?); it will expire by TTL");
            None
        }
    }
}

/// Phase 7: mirror device state into `spots_by_zone` when `SPOTS_BY_ZONE=on`.
async fn zone_write(
    state: &AppState,
    level: &str,
    spot_id: &str,
    occupied: Option<(bool, Option<&str>)>,
    light: Light,
    ts: DateTime<Utc>,
) -> anyhow::Result<()> {
    if !state.spots_by_zone {
        return Ok(());
    }
    spots::update_zone_state(&state.db, level, spot_id, occupied, light, ts.timestamp_micros()).await
}

/// CAR LEAVES THE SPOT.
async fn free(state: &AppState, spot: &str, light: Light, ts: DateTime<Utc>) -> ApiResult<Value> {
    let (level, spot_id) = split_spot_id(spot)?;
    let db = &state.db;

    tokio::try_join!(
        spots::update_spot_state(db, level, spot_id, false, None, light, ts.timestamp_micros()),
        spots::record_spot_event(db, spot_id, ts, "free", None, light),
        counters::add_free(db, level, 1),
        zone_write(state, level, spot_id, Some((false, None)), light, ts),
    )
    .map_err(internal)?;
    Ok(json!({ "stored": "free" }))
}

/// DEVICE HEARTBEAT: refresh the 60s liveness row, store what the light shows,
/// and reply with the override the server wants.
async fn heartbeat(
    state: &AppState,
    spot: &str,
    light: Light,
    battery: u8,
    ts: DateTime<Utc>,
) -> ApiResult<Value> {
    let (level, spot_id) = split_spot_id(spot)?;
    let db = &state.db;

    let (_, _, current, _) = tokio::try_join!(
        heartbeats::record(db, level, spot_id, battery, light, ts),
        spots::report_light(db, level, spot_id, light, ts.timestamp_micros()),
        spots::get_spot(db, level, spot_id),
        zone_write(state, level, spot_id, None, light, ts),
    )
    .map_err(internal)?;

    let override_color = current.and_then(|spot| spot.light_override);
    Ok(json!({ "override": override_color }))
}

/// CAR ENTERS. A duplicate camera read finds the session it just created and
/// does nothing.
///
/// Known gap: an entry event delivered *after* that visit's exit would open a
/// new session, because nothing remembers that the visit already ended.
/// Guarding against that would need a per-plate "last exit" record. The sim
/// only delays spot events, never gate reads.
async fn gate_in(state: &AppState, gate: &str, plate: &str, ts: DateTime<Utc>) -> ApiResult<Value> {
    let outcome = sessions::open(&state.db, plate, ts, gate)
        .await
        .map_err(internal)?;

    Ok(match outcome {
        sessions::Entry::Opened => {
            tracing::info!(%plate, %gate, "session opened");
            json!({ "session": "opened" })
        }
        sessions::Entry::Duplicate => {
            tracing::info!(%plate, %gate, "duplicate entry read ignored");
            json!({ "session": "duplicate" })
        }
        sessions::Entry::AlreadyOpen { entered_at } => {
            tracing::warn!(%plate, %gate, %entered_at, "car already has an open session; keeping it");
            json!({ "session": "already_open", "entered_at": entered_at })
        }
    })
}

/// CAR EXITS: read the session (and any monthly pass), bill it, record it,
/// then delete it.
///
/// The order matters. If parkd dies after recording but before deleting, the
/// camera's retry finds the session again and rewrites the *same*
/// `sessions_by_day` row (same day, exit time and plate). Deleting first would
/// lose the visit instead.
async fn gate_out(
    state: &AppState,
    gate: &str,
    plate: &str,
    ts: DateTime<Utc>,
) -> ApiResult<Value> {
    let db = &state.db;
    let (active, pass) = tokio::try_join!(
        sessions::get_active(db, plate),
        passes::get(db, plate),
    )
    .map_err(internal)?;
    let Some(active) = active else {
        // A duplicate exit read after the session closed, or a car that was
        // never seen at the entry.
        tracing::info!(%plate, %gate, "exit with no open session ignored");
        return Ok(json!({ "session": "none" }));
    };

    if ts <= active.entered_at {
        // A late exit event from a previous visit. The current visit started
        // after it, so it can't be what ends this one.
        tracing::warn!(%plate, %gate, exit = %ts, entered_at = %active.entered_at, "stale exit event ignored");
        return Ok(json!({ "session": "stale_exit" }));
    }

    let stay = ts - active.entered_at;
    // A pass covers the visit if it is still valid when the car leaves.
    let pass_holder = pass.filter(|pass| pass.covers(ts)).map(|pass| pass.holder);
    let fee = if pass_holder.is_some() { 0 } else { fee_cents(stay) };
    sessions::record_closed(db, plate, active.entered_at, ts, stay.num_seconds(), fee)
        .await
        .map_err(internal)?;
    let (closed, _) = tokio::try_join!(
        sessions::close(db, plate, active.entered_at),
        spots::delete_car_location(db, plate, ts),
    )
    .map_err(internal)?;

    tracing::info!(%plate, %gate, stay_seconds = stay.num_seconds(), fee_cents = fee, ?pass_holder, closed, "session closed");
    Ok(json!({
        "session": "closed",
        "entered_at": active.entered_at,
        "exited_at": ts,
        "duration_seconds": stay.num_seconds(),
        "fee_cents": fee,
        "pass_holder": pass_holder,
    }))
}
