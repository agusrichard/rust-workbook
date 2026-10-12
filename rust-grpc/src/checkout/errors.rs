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
