use crate::response::MessageResponse;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    InternalError(String),
}

impl AppError {
    pub fn not_found(entity: &str, id: u64) -> Self {
        AppError::NotFound(format!("{} {} not found", entity, id))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(MessageResponse { message })).into_response()
    }
}
