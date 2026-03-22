use axum::{Json, Router};
use axum::response::IntoResponse;
use axum::routing::get;
use crate::app_state::AppState;
use crate::response::MessageResponse;

pub mod todo;

pub async fn hello_world() -> impl IntoResponse {
    Json(MessageResponse {
        message: "Hello World!".to_string()
    })
}

pub fn app(state: AppState) -> Router {
    Router::new().route("/", get(hello_world))
        .nest("/todos", todo::router()).with_state(state)
}