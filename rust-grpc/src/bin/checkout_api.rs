use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use rust_grpc::inventory_v1::GetProductRequest;
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
struct ProductResponse {
    product_id: i64,
    name: String,
    stock: i64,
}

async fn get_product(
    State(state): State<AppState>,
    Path(product_id): Path<i64>,
) -> Result<Json<ProductResponse>, StatusCode> {
    let mut client = state.inventory_client.clone();
    let mut request = Request::new(GetProductRequest { product_id });

    request.set_timeout(Duration::from_secs(2));

    let product = client
        .get_product(request)
        .await
        .map_err(|status| match status.code() {
            tonic::Code::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        })?
        .into_inner();
    Ok(Json(ProductResponse {
        product_id: product.product_id,
        name: product.name,
        stock: product.stock,
    }))
}

async fn setup_server() -> Result<(), Box<dyn std::error::Error>> {
    let client = InventoryClient::connect("http://[::1]:50051").await?;

    let app = Router::new()
        .route("/products/{product_id}", get(get_product))
        .with_state(AppState {
            inventory_client: client,
        });

    let addr = "127.0.0.1:8080";
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
