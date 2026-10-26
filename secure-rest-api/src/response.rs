use std::eprintln;

use axum::{Json, http::StatusCode};
use axum_extra::extract::CookieJar;
use serde::Serialize;

#[derive(Serialize)]
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

pub fn api_error(status: StatusCode, message: &str, log_message: Option<&str>) -> ApiError {
    if let Some(msg) = log_message {
        eprintln!("Error with status_code={} log_message={}", status, msg);
    }
    (status, Json(ErrorResponse::new(message)))
}

pub type ApiError = (StatusCode, Json<ErrorResponse>);
pub type ApiResult<T> = Result<(StatusCode, CookieJar, Json<T>), ApiError>;
