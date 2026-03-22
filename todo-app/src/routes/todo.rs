use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{Json, Router, routing::{get, post, put, delete}};
use crate::app_state::AppState;
use crate::response::MessageResponse;

async fn create_todo() -> impl IntoResponse {
    Json(MessageResponse {
        message: "Create Todo".to_string()
    })
}

async fn get_todos() -> impl IntoResponse {
    Json(MessageResponse {
        message: "Get Todos".to_string()
    })
}

async fn get_todo(Path(todo_id): Path<u64>) -> impl IntoResponse {
    Json(MessageResponse {
        message: format!("Get Todo {}", todo_id)
    })
}


async fn update_todo(Path(todo_id): Path<u64>) -> impl IntoResponse {
    Json(MessageResponse {
        message: format!("Update Todo {}", todo_id)
    })
}


async fn delete_todo(Path(todo_id): Path<u64>) -> impl IntoResponse {
    Json(MessageResponse {
        message: format!("Delete Todo {}", todo_id)
    })
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_todos))
        .route("/", post(create_todo))
        .route("/{todo_id}", get(get_todo))
        .route("/{todo_id}", put(update_todo))
        .route("/{todo_id}", delete(delete_todo))
}