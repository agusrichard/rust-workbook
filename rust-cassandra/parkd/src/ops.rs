//! Operator endpoints: maintenance, offline devices, broken lights, daily report.

use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::{NaiveDate, Utc};
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::AppState;
use crate::db::{self, heartbeats, reservations, sessions, spots};
use crate::error::{ApiResult, bad_request, internal, not_found, split_spot_id};
use crate::lights::{Light, Override, expected_light};

/// Longest maintenance window accepted: 30 days.
const MAX_MAINTENANCE_HOURS: f64 = 30.0 * 24.0;

#[derive(Debug, Deserialize)]
pub struct MaintenanceRequest {
    on: bool,
    /// Fractions are allowed, so `0.01` (36 seconds) is a quick TTL test.
    #[serde(default = "default_hours")]
    hours: f64,
}

fn default_hours() -> f64 {
    2.0
}

/// OPERATOR SETS MAINTENANCE.
pub async fn maintenance(
    State(state): State<AppState>,
    Path(spot): Path<String>,
    Json(request): Json<MaintenanceRequest>,
) -> ApiResult<Json<Value>> {
    let (level, spot_id) = split_spot_id(&spot)?;
    let db = &state.db;

    if spots::get_spot(db, level, spot_id)
        .await
        .map_err(internal)?
        .is_none()
    {
        return Err(not_found(format!("unknown spot {spot}")));
    }

    if !request.on {
        // Ending early: write null over the override. Only touch it if it's
        // still "off", so this never removes a reservation's blue.
        let ended = spots::clear_override_if(db, level, spot_id, Override::Off)
            .await
            .map_err(internal)?;
        tracing::info!(%spot, ended, "maintenance ended");
        return Ok(Json(json!({ "spot": spot, "maintenance": false })));
    }

    if !(request.hours > 0.0 && request.hours <= MAX_MAINTENANCE_HOURS) {
        return Err(bad_request(format!(
            "hours must be greater than 0 and at most {MAX_MAINTENANCE_HOURS}"
        )));
    }
    let ttl_seconds = (request.hours * 3600.0).ceil() as i32;

    // A spot under maintenance can't stay reserved. Cancel first, then switch
    // the light off, so the driver's reservation can't turn it blue again.
    let reservation_cancelled = reservations::drop_any(db, spot_id)
        .await
        .map_err(internal)?;
    spots::set_override(db, level, spot_id, Override::Off, ttl_seconds)
        .await
        .map_err(internal)?;
    tracing::info!(%spot, ttl_seconds, reservation_cancelled, "maintenance started");

    Ok(Json(json!({
        "spot": spot,
        "maintenance": true,
        "expires_in_seconds": ttl_seconds,
        "reservation_cancelled": reservation_cancelled,
    })))
}

#[derive(Debug, Serialize)]
pub struct LevelOffline {
    level: String,
    total: usize,
    online: usize,
    offline: Vec<String>,
}

/// A spot is offline when its heartbeat row has expired: every spot on the
/// level, minus the ones with a row in `sensor_heartbeats`.
pub async fn offline(State(state): State<AppState>) -> ApiResult<Json<Vec<LevelOffline>>> {
    let db = &state.db;
    let levels = try_join_all(db::LEVELS.iter().map(|level| async move {
        let (all, online) = tokio::try_join!(
            spots::spots_in_level(db, level),
            heartbeats::online_spots(db, level),
        )?;
        let offline: Vec<String> = all
            .iter()
            .filter(|spot| !online.contains(&spot.spot_id))
            .map(|spot| spot.spot_id.clone())
            .collect();
        anyhow::Ok(LevelOffline {
            level: (*level).to_owned(),
            total: all.len(),
            online: all.len() - offline.len(),
            offline,
        })
    }))
    .await
    .map_err(internal)?;
    Ok(Json(levels))
}

#[derive(Debug, Serialize)]
pub struct Fault {
    spot: String,
    expected: Light,
    reported: Light,
    occupied: bool,
    light_override: Option<Override>,
}

/// Compares what the server wants (worked out from `occupied` and
/// `light_override`) with what the unit says its light shows.
///
/// A unit learns about a new override on its next heartbeat, so a spot can
/// show up here for up to 15 seconds after a reservation or maintenance
/// change. A fault that persists is a real one, such as a broken bulb.
pub async fn faults(State(state): State<AppState>) -> ApiResult<Json<Vec<Fault>>> {
    let db = &state.db;
    let levels = try_join_all(
        db::LEVELS
            .iter()
            .map(|level| spots::spots_in_level(db, level)),
    )
    .await
    .map_err(internal)?;

    let faults = levels
        .into_iter()
        .flatten()
        .filter_map(|spot| {
            let reported = spot.light_reported?;
            let expected = expected_light(spot.occupied, spot.light_override);
            if reported == expected {
                return None;
            }
            Some(Fault {
                spot: spot.spot_id,
                expected,
                reported,
                occupied: spot.occupied,
                light_override: spot.light_override,
            })
        })
        .collect();
    Ok(Json(faults))
}

#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    /// `YYYY-MM-DD`, in UTC. Defaults to today.
    day: Option<NaiveDate>,
}

/// Visits, average stay and revenue for one day: one partition of
/// `sessions_by_day`. A monthly report would read 30 partitions concurrently.
pub async fn report(
    State(state): State<AppState>,
    Query(query): Query<ReportQuery>,
) -> ApiResult<Json<sessions::DailyReport>> {
    let day = query.day.unwrap_or_else(|| Utc::now().date_naive());
    sessions::daily_report(&state.db, day)
        .await
        .map(Json)
        .map_err(internal)
}
