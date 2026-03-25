use crate::app_state::AppState;
use crate::repository::todo::in_memory::InMemoryTodoRepository;
use std::sync::Arc;

pub mod app_state;
pub mod errors;
pub mod models;
pub mod repository;
pub mod response;
pub mod routes;

#[tokio::main]
async fn main() {
    let v1_state = AppState {
        todo_repo: Arc::new(InMemoryTodoRepository::new()),
    };
    let v2_state = AppState {
        todo_repo: Arc::new(InMemoryTodoRepository::new()),
    };
    let app = routes::app(v1_state, v2_state);

    let host = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(host).await.unwrap();
    println!("Listing on {host}");
    axum::serve(listener, app).await.unwrap();
}
