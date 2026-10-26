pub mod auth;
pub mod db;
pub mod handlers;
pub mod models;
pub mod response;

use std::println;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::{Pool, Sqlite};

use crate::handlers::{hello_world, login, register};

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Sqlite>,
    access_token_secret: String,
    refresh_token_secret: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let api_url = std::env::var("API_URL").expect("API_URL is not set");
    let access_token_secret =
        std::env::var("ACCESS_TOKEN_SECRET").expect("ACCESS_TOKEN_SECRET is not set");
    let refresh_token_secret =
        std::env::var("REFRESH_TOKEN_SECRET").expect("REFRESH_TOKEN_SECRET is not set");
    let pool = db::connect(&database_url).await?;

    let app = Router::new()
        .route("/", get(hello_world))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .with_state(AppState {
            pool,
            access_token_secret,
            refresh_token_secret,
        });

    let listener = tokio::net::TcpListener::bind(&api_url).await?;
    println!("Server is running {}", &api_url);
    axum::serve(listener, app).await?;

    Ok(())
}
