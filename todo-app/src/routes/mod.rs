use axum::Json;
use axum::response::IntoResponse;
use crate::response::MessageResponse;

pub mod todo;

pub async fn hello_world() -> impl IntoResponse {
    Json(MessageResponse {
        message: "Hello World!".to_string()
    })
}