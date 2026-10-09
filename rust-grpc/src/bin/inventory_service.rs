use dotenvy::dotenv;
use rust_grpc::inventory::repository::ProductRepository;
use rust_grpc::inventory_v1::inventory_server::{Inventory, InventoryServer};
use rust_grpc::inventory_v1::{GetAvailabilityRequest, GetAvailabilityResponse};
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::env;
use std::str::FromStr;
use tonic::Response;
use tonic::transport::Server;

const DATABASE_URL: &str = "sqlite://data/stockguard.db";

struct InventoryService {
    repository: ProductRepository,
}

impl InventoryService {
    fn new(pool: SqlitePool) -> Self {
        let repository = ProductRepository::new(pool);
        Self { repository }
    }
}

#[tonic::async_trait]
impl Inventory for InventoryService {
    async fn get_availability(
        &self,
        request: tonic::Request<GetAvailabilityRequest>,
    ) -> tonic::Result<Response<GetAvailabilityResponse>, tonic::Status> {
        let id = request.into_inner().product_id;

        let product = self
            .repository
            .get_by_id(id)
            .await
            .map_err(|e| tonic::Status::internal(format!("Database error: {}", e)))?;

        match product {
            Some(product) => Ok(Response::new(GetAvailabilityResponse {
                product_id: product.id,
                name: product.name,
                available_quantity: product.stock,
            })),
            None => Err(tonic::Status::not_found("not found")),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").unwrap_or(DATABASE_URL.to_string());

    let connection_options = SqliteConnectOptions::from_str(db_url.as_str())?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePool::connect_with(connection_options).await?;
    sqlx::migrate!().run(&pool).await?;
    println!("Migrations applied successfully!");

    let address = "[::1]:50051".parse()?;
    let service = InventoryService::new(pool);
    println!("Inventory server listening on {address}");
    Server::builder()
        .add_service(InventoryServer::new(service))
        .serve(address)
        .await?;

    Ok(())
}
