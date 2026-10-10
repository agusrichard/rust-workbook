use axum::Router;
use axum::routing::get;
use rust_grpc::checkout::handlers::{AppState, get_product};
use rust_grpc::inventory_v1::inventory_client::InventoryClient;

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
