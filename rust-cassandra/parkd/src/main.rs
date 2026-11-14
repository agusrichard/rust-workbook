//! parkd: the ParkWatch Mall HTTP service.
//!
//! `parkd --init-schema` creates the keyspace and tables, then exits.
//! `parkd` serves device ingest plus the driver and operator APIs.

mod db;
mod drivers;
mod error;
mod events;
mod fees;
mod lights;
mod map;
mod ops;

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

use crate::db::{Db, counters, spots};
use crate::error::{ApiResult, bad_request, internal};

#[derive(Clone)]
pub struct AppState {
    db: Arc<Db>,
    /// How long a reservation (and its blue light) lasts. 900 in production;
    /// set `RESERVATION_TTL_SECONDS=20` to watch one expire quickly.
    reservation_ttl_seconds: i32,
    /// Phase 7: also write device state into `spots_by_zone`
    /// (`SPOTS_BY_ZONE=on`), to compare partition load.
    spots_by_zone: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // e.g. CASSANDRA_NODES=cassandra1:9042,cassandra2:9042
    let contact_points: Vec<String> = std::env::var("CASSANDRA_NODES")
        .unwrap_or_else(|_| "cassandra1:9042".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let listen_addr = std::env::var("PARKD_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let reservation_ttl_seconds: i32 = match std::env::var("RESERVATION_TTL_SECONDS") {
        Ok(value) => value
            .parse()
            .context("RESERVATION_TTL_SECONDS must be a whole number of seconds")?,
        Err(_) => 900,
    };
    anyhow::ensure!(
        reservation_ttl_seconds > 0,
        "RESERVATION_TTL_SECONDS must be positive"
    );

    let spots_by_zone = std::env::var("SPOTS_BY_ZONE").is_ok_and(|value| value == "on");

    tracing::info!(?contact_points, "connecting to Cassandra");
    let session = db::connect(&contact_points).await?;

    let info = db::cluster_info(&session).await?;
    tracing::info!(
        cluster = %info.cluster_name,
        version = %info.release_version,
        nodes = info.nodes.len(),
        "connected"
    );
    for node in &info.nodes {
        tracing::info!(
            address = %node.address,
            dc = node.datacenter.as_deref().unwrap_or("?"),
            rack = node.rack.as_deref().unwrap_or("?"),
            connected = node.connected,
            "discovered node"
        );
    }

    if std::env::args()
        .skip(1)
        .any(|argument| argument == "--init-schema")
    {
        db::init_schema(&session).await?;
        tracing::info!("ParkWatch schema is ready");
        return Ok(());
    }

    let state = AppState {
        db: Arc::new(Db::from(session, 256)),
        reservation_ttl_seconds,
        spots_by_zone,
    };
    let app = Router::new()
        .route("/health", get(health))
        // Devices
        .route("/events", post(events::ingest))
        // Drivers and entrance screens
        .route("/levels/free", get(drivers::levels_free))
        .route("/cars/{plate}", get(drivers::find_car))
        .route(
            "/spots/{id}/reserve",
            post(drivers::reserve).delete(drivers::cancel),
        )
        // Operators
        .route("/spots/{id}/maintenance", put(ops::maintenance))
        .route("/ops/offline", get(ops::offline))
        .route("/ops/faults", get(ops::faults))
        .route("/ops/report", get(ops::report))
        .route("/ops/counters", get(ops::counters))
        .route(
            "/passes/{plate}",
            put(ops::put_pass).get(ops::get_pass).delete(ops::delete_pass),
        )
        // Live map
        .route("/map", get(map::page))
        .route("/spots", get(map::all_spots))
        // This is an internal bootstrap route used by `sim --seed`, rather
        // than an operator-facing part of the public API.
        .route("/internal/seed", post(seed))
        .route(
            "/internal/spots/{id}/reserve-without-lwt",
            post(drivers::reserve_without_lwt),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
    tracing::info!(%listen_addr, reservation_ttl_seconds, spots_by_zone, "parkd listening");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Cluster name, Cassandra version and every node the driver knows about.
async fn health(State(state): State<AppState>) -> ApiResult<Json<db::ClusterInfo>> {
    db::cluster_info(state.db.get_session())
        .await
        .map(Json)
        .map_err(|err| (StatusCode::SERVICE_UNAVAILABLE, err.to_string()))
}

#[derive(Debug, Deserialize)]
struct SeedRequest {
    #[serde(default = "default_spots_per_level")]
    spots_per_level: u16,
}

fn default_spots_per_level() -> u16 {
    40
}

#[derive(Debug, Serialize)]
struct SeedResponse {
    spots: usize,
}

async fn seed(
    State(state): State<AppState>,
    Json(request): Json<SeedRequest>,
) -> ApiResult<Json<SeedResponse>> {
    if request.spots_per_level == 0 {
        return Err(bad_request("spots_per_level must be positive"));
    }

    let mut spots = 0;
    for level in db::LEVELS {
        for number in 1..=request.spots_per_level {
            let spot_id = format!("{level}-{number:02}");
            spots::seed_spot(
                &state.db,
                level,
                &spot_id,
                &format!("Level {level}"),
                i32::from(request.spots_per_level),
            )
            .await
            .map_err(internal)?;
            spots += 1;
        }
        // Every spot is empty again, so the counter starts at full capacity.
        counters::reset_free(&state.db, level, i64::from(request.spots_per_level))
            .await
            .map_err(internal)?;
    }
    tracing::info!(spots, "seeded simulator spots");
    Ok(Json(SeedResponse { spots }))
}
