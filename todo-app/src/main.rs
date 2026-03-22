use std::sync::Arc;
use crate::app_state::AppState;
use crate::repository::todo::in_memory::InMemoryTodoRepository;

pub mod routes;
pub mod models;
pub mod repository;
pub mod response;
pub mod app_state;

#[tokio::main]
async fn main() {
    let app_state = AppState {
        todo_repo: Arc::new(InMemoryTodoRepository::new()),
    };
    let app = routes::app(app_state);

    let host = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(host).await.unwrap();
    println!("Listing on {host}");
    axum::serve(listener, app).await.unwrap();
}