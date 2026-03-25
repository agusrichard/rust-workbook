use crate::app_state::AppState;
use crate::response::MessageResponse;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};

pub mod todo;

pub async fn hello_world() -> impl IntoResponse {
    Json(MessageResponse {
        message: "Hello World!".to_string(),
    })
}

pub fn app(v1_state: AppState, v2_state: AppState) -> Router {
    let v1 = todo::router().with_state(v1_state);
    let v2 = todo::router().with_state(v2_state);

    Router::new()
        .route("/", get(hello_world))
        .nest("/api/v1/todos", v1)
        .nest("/api/v2/todos", v2)
}
