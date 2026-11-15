//! What drivers and entrance screens use: free spots, find my car, reservations.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Utc;
use futures::future::try_join_all;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::AppState;
use crate::db::{self, reservations, spots};
use crate::error::{ApiResult, bad_request, conflict, internal, not_found, split_spot_id};
use crate::lights::Override;

/// Reads the level partitions concurrently, never the whole table.
pub async fn levels_free(State(state): State<AppState>) -> ApiResult<Json<Vec<spots::FreeSpots>>> {
    let levels = try_join_all(
        db::LEVELS
            .iter()
            .map(|level| spots::free_spots_in_level(&state.db, level)),
    )
    .await
    .map_err(internal)?;
    Ok(Json(levels))
}

pub async fn find_car(
    State(state): State<AppState>,
    Path(plate): Path<String>,
) -> ApiResult<Json<spots::CarLocation>> {
    spots::get_car_location(&state.db, &plate)
        .await
        .map_err(internal)?
        .map(Json)
        .ok_or_else(|| not_found(format!("no parked car with plate {plate}")))
}

#[derive(Debug, Deserialize)]
pub struct ReserveRequest {
    /// Who holds the reservation, usually the driver's plate.
    reserved_by: String,
}

fn validate_holder(reserved_by: &str) -> ApiResult<()> {
    if reserved_by.trim().is_empty() || reserved_by.len() > 32 {
        return Err(bad_request("reserved_by must be 1 to 32 characters"));
    }
    Ok(())
}

/// DRIVER RESERVES A SPOT.
pub async fn reserve(
    State(state): State<AppState>,
    Path(spot): Path<String>,
    Json(request): Json<ReserveRequest>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let (level, spot_id) = split_spot_id(&spot)?;
    validate_holder(&request.reserved_by)?;
    let db = &state.db;
    let ttl = state.reservation_ttl_seconds;

    let current = spots::get_spot(db, level, spot_id)
        .await
        .map_err(internal)?
        .ok_or_else(|| not_found(format!("unknown spot {spot}")))?;
    if current.light_override == Some(Override::Off) {
        return Err(conflict(format!("{spot} is under maintenance")));
    }
    if current.occupied {
        return Err(conflict(format!("{spot} is taken")));
    }

    // The check above can't stop two drivers who both saw a free spot. The
    // LWT can: exactly one INSERT ... IF NOT EXISTS applies.
    let outcome = reservations::reserve(db, spot_id, &request.reserved_by, Utc::now(), ttl)
        .await
        .map_err(internal)?;
    let status = match outcome {
        reservations::Reserve::Reserved => StatusCode::CREATED,
        reservations::Reserve::AlreadyYours => StatusCode::OK,
        reservations::Reserve::Taken => {
            return Err(conflict(format!("someone got {spot} first")));
        }
    };

    // Same TTL as the reservation row, so the light goes back to green on
    // its own when the reservation expires.
    spots::set_override(db, level, spot_id, Override::Blue, ttl)
        .await
        .map_err(internal)?;
    tracing::info!(%spot, reserved_by = %request.reserved_by, ttl, "spot reserved");

    Ok((
        status,
        Json(json!({
            "spot": spot,
            "reserved_by": request.reserved_by,
            "expires_in_seconds": ttl,
        })),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CancelQuery {
    reserved_by: String,
}

/// `DELETE /spots/{id}/reserve?reserved_by=...`: only the holder can cancel.
pub async fn cancel(
    State(state): State<AppState>,
    Path(spot): Path<String>,
    Query(query): Query<CancelQuery>,
) -> ApiResult<StatusCode> {
    let (level, spot_id) = split_spot_id(&spot)?;
    validate_holder(&query.reserved_by)?;
    let db = &state.db;

    match reservations::cancel(db, spot_id, &query.reserved_by)
        .await
        .map_err(internal)?
    {
        reservations::Cancel::Cancelled => {
            spots::clear_override_if(db, level, spot_id, Override::Blue)
                .await
                .map_err(internal)?;
            tracing::info!(%spot, reserved_by = %query.reserved_by, "reservation cancelled");
            Ok(StatusCode::NO_CONTENT)
        }
        reservations::Cancel::NotHolder => Err((
            StatusCode::FORBIDDEN,
            format!("{spot} is reserved by someone else"),
        )),
        reservations::Cancel::NotFound => Err(not_found(format!("{spot} has no reservation"))),
    }
}

/// `POST /internal/spots/{id}/reserve-without-lwt`: the Phase 4 race test's
/// broken variant (read, then insert if empty). Answers like `reserve` does,
/// 201 or 409, so `sim --race-without-lwt` can count how many drivers were
/// told they won. It doesn't touch the light.
pub async fn reserve_without_lwt(
    State(state): State<AppState>,
    Path(spot): Path<String>,
    Json(request): Json<ReserveRequest>,
) -> ApiResult<StatusCode> {
    let (_, spot_id) = split_spot_id(&spot)?;
    validate_holder(&request.reserved_by)?;
    let won = reservations::reserve_without_lwt(
        &state.db,
        spot_id,
        &request.reserved_by,
        Utc::now(),
        state.reservation_ttl_seconds,
    )
    .await
    .map_err(internal)?;
    if won {
        Ok(StatusCode::CREATED)
    } else {
        Err(conflict(format!("someone got {spot} first")))
    }
}
