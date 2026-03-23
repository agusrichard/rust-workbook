use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use crate::response::MessageResponse;

pub enum AppError {
    NotFound(String),
    InternalError(String)
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