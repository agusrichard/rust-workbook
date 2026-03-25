use crate::app_state::AppState;
use crate::repository::todo::in_memory::InMemoryTodoRepository;
use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use crate::repository::todo::postgres::PostgresTodoRepository;

pub mod app_state;
pub mod errors;
pub mod models;
pub mod repository;
pub mod response;
pub mod routes;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to Postgres DB");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let v1_state = AppState {
        todo_repo: Arc::new(InMemoryTodoRepository::new()),
    };
    let v2_state = AppState {
        todo_repo: Arc::new(PostgresTodoRepository::new(pool)),
    };
    let app = routes::app(v1_state, v2_state);

    let host = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(host).await.unwrap();
    println!("Listing on {host}");
    axum::serve(listener, app).await.unwrap();
}
