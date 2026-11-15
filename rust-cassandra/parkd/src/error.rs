//! HTTP error responses shared by every handler.

use axum::http::StatusCode;

pub type ApiResult<T> = Result<T, (StatusCode, String)>;

pub fn bad_request(message: impl Into<String>) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, message.into())
}

pub fn not_found(message: impl Into<String>) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, message.into())
}

pub fn conflict(message: impl Into<String>) -> (StatusCode, String) {
    (StatusCode::CONFLICT, message.into())
}

/// Any Cassandra failure. 503 rather than 500, because the usual cause is
/// that not enough replicas answered (for example, LWT with 2 nodes down).
pub fn internal(error: anyhow::Error) -> (StatusCode, String) {
    tracing::error!(error = format!("{error:#}"), "database operation failed");
    (StatusCode::SERVICE_UNAVAILABLE, format!("{error:#}"))
}

/// Splits `B2-17` into the level partition key (`B2`) and the full spot id.
pub fn split_spot_id(spot: &str) -> ApiResult<(&str, &str)> {
    match spot.split_once('-') {
        Some((level, number)) if !level.is_empty() && !number.is_empty() => Ok((level, spot)),
        _ => Err(bad_request(format!(
            "spot must have the form LEVEL-NUMBER, got {spot:?}"
        ))),
    }
}
