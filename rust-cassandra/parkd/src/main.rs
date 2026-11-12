//! parkd: the ParkWatch Mall HTTP service.
//!
//! It owns schema initialization and Phase 2's device-event ingest API.

mod db;

use std::sync::Arc;

use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use scylla::client::session::Session;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    session: Arc<Session>,
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
        session: Arc::new(session),
    };
    let app = Router::new()
        .route("/health", get(health))
        .route("/events", post(ingest_event))
        .route("/levels/free", get(levels_free))
        // This is an internal bootstrap route used by `sim --seed`, rather
        // than an operator-facing part of the public API.
        .route("/internal/seed", post(seed))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
    tracing::info!(%listen_addr, "parkd listening");
    axum::serve(listener, app).await?;
    Ok(())
}

type ApiResult<T> = Result<T, (StatusCode, String)>;

fn internal_error(error: anyhow::Error) -> (StatusCode, String) {
    tracing::error!(%error, "database operation failed");
    (StatusCode::SERVICE_UNAVAILABLE, error.to_string())
}

/// Cluster name, Cassandra version and every node the driver knows about.
async fn health(State(state): State<AppState>) -> ApiResult<Json<db::ClusterInfo>> {
    db::cluster_info(&state.session)
        .await
        .map(Json)
        .map_err(|err| (StatusCode::SERVICE_UNAVAILABLE, err.to_string()))
}

/// The wire format sent by a spot unit or gate camera. Keeping it as a tagged
/// enum prevents impossible combinations such as a `free` event with a plate.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum DeviceEvent {
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
        direction: String,
        plate: String,
        ts: DateTime<Utc>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Light {
    Green,
    Red,
    Blue,
    Off,
}

impl Light {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Green => "green",
            Self::Red => "red",
            Self::Blue => "blue",
            Self::Off => "off",
        }
    }
}

async fn ingest_event(
    State(state): State<AppState>,
    Json(event): Json<DeviceEvent>,
) -> ApiResult<StatusCode> {
    match event {
        DeviceEvent::Occupied {
            spot,
            plate,
            light,
            ts,
        } => {
            let (level, spot_id) = split_spot_id(&spot)?;
            db::update_spot_state(
                &state.session,
                level,
                spot_id,
                true,
                Some(&plate),
                light.as_str(),
                ts.timestamp_micros(),
            )
            .await
            .map_err(internal_error)?;
            tracing::debug!(%spot, %plate, timestamp_micros = ts.timestamp_micros(), "stored occupied event");
        }
        DeviceEvent::Free { spot, light, ts } => {
            let (level, spot_id) = split_spot_id(&spot)?;
            db::update_spot_state(
                &state.session,
                level,
                spot_id,
                false,
                None,
                light.as_str(),
                ts.timestamp_micros(),
            )
            .await
            .map_err(internal_error)?;
            tracing::debug!(%spot, timestamp_micros = ts.timestamp_micros(), "stored free event");
        }
        // Heartbeats and gates become persistent features in later phases. We
        // still parse their defined wire formats now, so device clients get a
        // stable event contract from Phase 2 onward.
        DeviceEvent::Heartbeat {
            spot,
            light,
            battery,
            ts,
        } => {
            tracing::debug!(%spot, light = light.as_str(), battery, %ts, "heartbeat accepted (persistence arrives in Phase 5)");
        }
        DeviceEvent::Gate {
            gate,
            direction,
            plate,
            ts,
        } => {
            tracing::debug!(%gate, %direction, %plate, %ts, "gate event accepted (sessions arrive in Phase 3)");
        }
    }
    Ok(StatusCode::ACCEPTED)
}

fn split_spot_id(spot: &str) -> ApiResult<(&str, &str)> {
    let Some((level, number)) = spot.split_once('-') else {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("spot must have the form LEVEL-NUMBER, got {spot:?}"),
        ));
    };
    if level.is_empty() || number.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("spot must have the form LEVEL-NUMBER, got {spot:?}"),
        ));
    }
    Ok((level, spot))
}

async fn levels_free(State(state): State<AppState>) -> ApiResult<Json<Vec<db::FreeSpots>>> {
    let (b1, b2, b3) = tokio::try_join!(
        db::free_spots_in_level(&state.session, db::LEVELS[0]),
        db::free_spots_in_level(&state.session, db::LEVELS[1]),
        db::free_spots_in_level(&state.session, db::LEVELS[2]),
    )
    .map_err(internal_error)?;
    Ok(Json(vec![b1, b2, b3]))
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
        return Err((
            StatusCode::BAD_REQUEST,
            "spots_per_level must be positive".into(),
        ));
    }

    let mut spots = 0;
    for level in db::LEVELS {
        for number in 1..=request.spots_per_level {
            let spot_id = format!("{level}-{number:02}");
            db::seed_spot(
                &state.session,
                level,
                &spot_id,
                &format!("Level {level}"),
                i32::from(request.spots_per_level),
            )
            .await
            .map_err(internal_error)?;
            spots += 1;
        }
    }
    tracing::info!(spots, "seeded simulator spots");
    Ok(Json(SeedResponse { spots }))
}
