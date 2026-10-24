pub mod auth;
pub mod db;
pub mod handlers;
pub mod models;
pub mod response;

use std::println;

use axum::{Router, routing::get};
use sqlx::{Pool, Sqlite};

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Sqlite>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let pool = db::connect(&database_url).await?;

    let app = Router::new()
        .route("/", get(handlers::hello_world))
        .with_state(AppState { pool });

    let listener = tokio::net::TcpListener::bind(&database_url).await?;
    println!("Server is running {database_url}");
    axum::serve(listener, app).await?;

    Ok(())
}
