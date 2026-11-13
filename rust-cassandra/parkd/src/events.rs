//! `POST /events`: everything spot units and gate cameras send.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::AppState;
use crate::db::{heartbeats, reservations, sessions, spots};
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

/// CAR PARKS: spot state, "find my car" and history are three independent
/// writes, so they run concurrently. All three use the device timestamp.
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
    )
    .map_err(internal)?;

    // If the spot was reserved, the reservation is used up now. Clear the
    // blue override only if our delete was the one that applied.
    let reservation_claimed = reservations::claim_on_arrival(db, spot_id, ts)
        .await
        .map_err(internal)?;
    if reservation_claimed {
        spots::clear_override_if(db, level, spot_id, Override::Blue)
            .await
            .map_err(internal)?;
        tracing::info!(%spot, %plate, "reserved spot taken, reservation cleared");
    }

    Ok(json!({ "stored": "occupied", "reservation_claimed": reservation_claimed }))
}

/// CAR LEAVES THE SPOT.
async fn free(state: &AppState, spot: &str, light: Light, ts: DateTime<Utc>) -> ApiResult<Value> {
    let (level, spot_id) = split_spot_id(spot)?;
    let db = &state.db;

    tokio::try_join!(
        spots::update_spot_state(db, level, spot_id, false, None, light, ts.timestamp_micros()),
        spots::record_spot_event(db, spot_id, ts, "free", None, light),
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

    let (_, _, current) = tokio::try_join!(
        heartbeats::record(db, level, spot_id, battery, light, ts),
        spots::report_light(db, level, spot_id, light, ts.timestamp_micros()),
        spots::get_spot(db, level, spot_id),
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

/// CAR EXITS: read the session, bill it, record it, then delete it.
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
    let Some(active) = sessions::get_active(db, plate).await.map_err(internal)? else {
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
    let fee = fee_cents(stay);
    sessions::record_closed(db, plate, active.entered_at, ts, stay.num_seconds(), fee)
        .await
        .map_err(internal)?;
    let (closed, _) = tokio::try_join!(
        sessions::close(db, plate, active.entered_at),
        spots::delete_car_location(db, plate, ts),
    )
    .map_err(internal)?;

    tracing::info!(%plate, %gate, stay_seconds = stay.num_seconds(), fee_cents = fee, closed, "session closed");
    Ok(json!({
        "session": "closed",
        "entered_at": active.entered_at,
        "exited_at": ts,
        "duration_seconds": stay.num_seconds(),
        "fee_cents": fee,
    }))
}
