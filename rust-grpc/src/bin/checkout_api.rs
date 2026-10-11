use axum::Router;
use axum::routing::{get, post};
use dotenvy::dotenv;
use rust_grpc::checkout::handlers::{
    AppState, create_purchase, get_product, watch_purchase_notifications,
};
use rust_grpc::checkout::subscriber::forward_purchase_events;
use rust_grpc::inventory_v1::inventory_client::InventoryClient;
use std::env;
use std::error::Error;
use tokio::sync::{broadcast, watch};
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint, Identity};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const HTTP_ADDRESS: &str = "127.0.0.1:8080";
const INVENTORY_ENDPOINT: &str = "https://[::1]:50051";
const INVENTORY_TLS_DOMAIN: &str = "inventory.local";
const CA_CERT_PATH: &str = "certs/ca.pem";
const CLIENT_CERT_PATH: &str = "certs/checkout.pem";
const CLIENT_KEY_PATH: &str = "certs/checkout-key.pem";
const EVENT_CHANNEL_CAPACITY: usize = 128;

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

fn inventory_channel() -> Result<Channel, Box<dyn Error>> {
    let endpoint =
        env::var("INVENTORY_GRPC_ENDPOINT").unwrap_or_else(|_| INVENTORY_ENDPOINT.to_string());
    let domain =
        env::var("INVENTORY_TLS_DOMAIN").unwrap_or_else(|_| INVENTORY_TLS_DOMAIN.to_string());
    let ca_path = env::var("TLS_CA_CERT").unwrap_or_else(|_| CA_CERT_PATH.to_string());
    let cert_path = env::var("CHECKOUT_TLS_CERT").unwrap_or_else(|_| CLIENT_CERT_PATH.to_string());
    let key_path = env::var("CHECKOUT_TLS_KEY").unwrap_or_else(|_| CLIENT_KEY_PATH.to_string());

    let ca = std::fs::read(ca_path)?;
    let cert = std::fs::read(cert_path)?;
    let key = std::fs::read(key_path)?;
    let tls = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(ca))
        .identity(Identity::from_pem(cert, key))
        .domain_name(domain);

    Ok(Endpoint::from_shared(endpoint)?
        .tls_config(tls)?
        .connect_lazy())
}

async fn shutdown_signal(shutdown: watch::Sender<bool>) {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("SIGTERM handler should install");
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result {
                    error!(%error, "failed to listen for Ctrl+C");
                }
            }
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        error!(%error, "failed to listen for Ctrl+C");
    }

    info!("checkout shutdown requested");
    let _ = shutdown.send(true);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    init_tracing();

    let address = env::var("CHECKOUT_HTTP_ADDRESS").unwrap_or_else(|_| HTTP_ADDRESS.to_string());
    let client = InventoryClient::new(inventory_channel()?);
    let (purchase_events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let subscriber = tokio::spawn(forward_purchase_events(
        client.clone(),
        purchase_events.clone(),
        shutdown_rx,
    ));

    let app = Router::new()
        .route("/products/{product_id}", get(get_product))
        .route("/purchases", post(create_purchase))
        .route(
            "/notifications/purchases",
            get(watch_purchase_notifications),
        )
        .with_state(AppState {
            inventory_client: client,
            purchase_events,
        });

    let listener = tokio::net::TcpListener::bind(&address).await?;
    info!(%address, "checkout HTTP server listening");

    let server_result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_tx.clone()))
        .await;
    let _ = shutdown_tx.send(true);

    if let Err(error) = subscriber.await {
        error!(%error, "purchase stream subscriber task failed");
    }

    server_result?;
    info!("checkout server stopped");
    Ok(())
}
