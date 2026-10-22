use axum::{Json, http::StatusCode};

use crate::models::User;

pub struct ErrorResponse {
    pub message: String,
}

impl ErrorResponse {
    pub fn new(message: &str) -> ErrorResponse {
        ErrorResponse {
            message: String::from(message),
        }
    }
}

pub type ApiError = (StatusCode, Json<ErrorResponse>);
pub type ApiResult<T> = Result<(StatusCode, Json<T>), ApiError>;

pub struct AuthPayload {
    pub token: String,
    pub user: User,
}
