use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{Json, Router, routing::{get, post, put, delete}};
use axum::http::StatusCode;
use crate::app_state::AppState;
use crate::errors::AppError;
use crate::models::todo::{CreateTodo, UpdateTodo};

async fn create_todo(State(state): State<AppState>, Json(body): Json<CreateTodo>) -> Result<impl IntoResponse, AppError> {
    let todo = state.todo_repo.create(body)?;
    Ok((StatusCode::CREATED, Json(todo)))
}

async fn get_todos(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let todos = state.todo_repo.get_all()?;
    Ok(Json(todos))
}

async fn get_todo(State(state): State<AppState>, Path(todo_id): Path<u64>) -> Result<impl IntoResponse, AppError> {
    let todo = state.todo_repo.get_todo(todo_id)?;
    Ok(Json(todo))
}


async fn update_todo(State(state): State<AppState>, Path(todo_id): Path<u64>, Json(body): Json<UpdateTodo>) -> Result<impl IntoResponse, AppError> {
    let todo = state.todo_repo.update(todo_id, body)?;
    Ok(Json(todo))
}


async fn delete_todo(State(state): State<AppState>, Path(todo_id): Path<u64>) -> Result<impl IntoResponse, AppError> {
    let todo = state.todo_repo.delete(todo_id)?;
    Ok(Json(todo))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_todos))
        .route("/", post(create_todo))
        .route("/{todo_id}", get(get_todo))
        .route("/{todo_id}", put(update_todo))
        .route("/{todo_id}", delete(delete_todo))
}