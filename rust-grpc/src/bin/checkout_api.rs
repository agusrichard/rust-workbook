use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use rust_grpc::inventory_v1::GetAvailabilityRequest;
use rust_grpc::inventory_v1::inventory_client::InventoryClient;
use serde::Serialize;
use std::time::Duration;
use tonic::Request;
use tonic::transport::Channel;

#[derive(Clone)]
struct AppState {
    inventory_client: InventoryClient<Channel>,
}

#[derive(Serialize)]
struct AvailabilityResponse {
    product_id: i64,
    name: String,
    availability_quantity: i64,
}

async fn hello_world() -> &'static str {
    "Hello, World!"
}

async fn get_inventory(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<AvailabilityResponse>, StatusCode> {
    let mut client = state.inventory_client.clone();
    let mut request = Request::new(GetAvailabilityRequest { product_id: id });

    request.set_timeout(Duration::from_secs(2));

    let product = client
        .get_availability(request)
        .await
        .map_err(|status| match status.code() {
            tonic::Code::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        })?
        .into_inner();
    Ok(Json(AvailabilityResponse {
        product_id: product.product_id,
        name: product.name,
        availability_quantity: product.available_quantity,
    }))
}

async fn setup_server() -> Result<(), Box<dyn std::error::Error>> {
    let client = InventoryClient::connect("http://[::1]:50051").await?;

    let app = Router::new()
        .route("/", get(hello_world))
        .route("/products/{id}/availability", get(get_inventory))
        .with_state(AppState {
            inventory_client: client,
        });

    let addr = "127.0.0.1:8000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("🚀 Checkout Server successfully running on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_server().await?;
    Ok(())
}
