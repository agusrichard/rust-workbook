//! parkd: the ParkWatch Mall HTTP service.
//!
//! Phase 0 only proves the plumbing works: connect to the 3-node cluster,
//! report what the driver discovered, and serve `GET /health`.

mod db;

use std::sync::Arc;

use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use scylla::client::session::Session;
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

    let state = AppState {
        session: Arc::new(session),
    };
    let app = Router::new()
        .route("/health", get(health))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
    tracing::info!(%listen_addr, "parkd listening");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Cluster name, Cassandra version and every node the driver knows about.
async fn health(
    State(state): State<AppState>,
) -> Result<Json<db::ClusterInfo>, (StatusCode, String)> {
    db::cluster_info(&state.session)
        .await
        .map(Json)
        .map_err(|err| (StatusCode::SERVICE_UNAVAILABLE, err.to_string()))
}
