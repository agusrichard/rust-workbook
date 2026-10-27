use std::eprintln;

use axum::{
    Json,
    http::{HeaderMap, HeaderValue, StatusCode, header::CACHE_CONTROL},
};
use axum_extra::extract::CookieJar;
use serde::Serialize;

#[derive(Debug, Serialize)]
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
pub type ApiResult<T> = Result<(StatusCode, HeaderMap, CookieJar, Json<T>), ApiError>;

pub fn no_store_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers
}
