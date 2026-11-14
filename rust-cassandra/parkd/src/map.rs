//! Phase 7 live map: `GET /map` serves a page that polls `GET /spots` every
//! 2 seconds and draws every spot as a coloured square.

use axum::Json;
use axum::extract::State;
use axum::response::Html;
use futures::future::try_join_all;
use serde::Serialize;

use crate::AppState;
use crate::db::{self, heartbeats, spots};
use crate::error::{ApiResult, internal};
use crate::lights::{Light, Override, expected_light};

pub async fn page() -> Html<&'static str> {
    Html(include_str!("map.html"))
}

#[derive(Debug, Serialize)]
pub struct LevelMap {
    level: String,
    free: usize,
    spots: Vec<SpotView>,
}

#[derive(Debug, Serialize)]
pub struct SpotView {
    spot: String,
    occupied: bool,
    plate: Option<String>,
    /// What the unit says its light shows.
    reported: Option<Light>,
    /// What the server works out it should show.
    expected: Light,
    light_override: Option<Override>,
    /// Has a heartbeat row, i.e. heard from in the last 60 seconds.
    online: bool,
}

/// Two reads per level (spots and heartbeats), all 6 concurrently.
pub async fn all_spots(State(state): State<AppState>) -> ApiResult<Json<Vec<LevelMap>>> {
    let db = &state.db;
    let levels = try_join_all(db::LEVELS.iter().map(|level| async move {
        let (all, online) = tokio::try_join!(
            spots::spots_in_level(db, level),
            heartbeats::online_spots(db, level),
        )?;
        let spots: Vec<SpotView> = all
            .into_iter()
            .map(|spot| SpotView {
                online: online.contains(&spot.spot_id),
                expected: expected_light(spot.occupied, spot.light_override),
                spot: spot.spot_id,
                occupied: spot.occupied,
                plate: spot.plate,
                reported: spot.light_reported,
                light_override: spot.light_override,
            })
            .collect();
        let free = spots
            .iter()
            .filter(|spot| !spot.occupied && spot.light_override.is_none())
            .count();
        anyhow::Ok(LevelMap {
            level: (*level).to_owned(),
            free,
            spots,
        })
    }))
    .await
    .map_err(internal)?;
    Ok(Json(levels))
}
