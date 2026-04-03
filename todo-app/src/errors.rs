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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_formats_message() {
        let err = AppError::not_found("Todo", 7);
        assert!(matches!(err, AppError::NotFound(_)));
        if let AppError::NotFound(msg) = err {
            assert_eq!(msg, "Todo 7 not found");
        }
    }

    #[test]
    fn internal_error_stores_message() {
        let err = AppError::InternalError("oops".to_string());
        assert!(matches!(err, AppError::InternalError(_)));
        if let AppError::InternalError(msg) = err {
            assert_eq!(msg, "oops");
        }
    }
}
