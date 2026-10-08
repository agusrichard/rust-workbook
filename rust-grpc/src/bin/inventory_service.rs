use dotenvy::dotenv;
use rust_grpc::inventory::repository::ProductRepository;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::env;
use std::str::FromStr;

const DATABASE_URL: &str = "sqlite://data/stockguard.db";

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").unwrap_or(DATABASE_URL.to_string());

    let connection_options = SqliteConnectOptions::from_str(db_url.as_str())?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePool::connect_with(connection_options).await?;
    sqlx::migrate!().run(&pool).await?;
    println!("Migrations applied successfully!");

    let product_repository = ProductRepository::new(&pool);
    let product = product_repository.get_by_id(1).await?;
    println!("Product {:?}", product);
    let products = product_repository.get_all().await?;
    println!("products {:?}", products);

    Ok(())
}
