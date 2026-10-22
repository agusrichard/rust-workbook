use axum::{extract::State, http::StatusCode};

use crate::{
    AppState,
    models::User,
    response::{ApiResult, AuthPayload},
};

pub async fn hello_world() -> String {
    "Hello World".to_string()
}

pub async fn register(State(state): State<AppState>) -> ApiResult<AuthPayload> {
    Ok((
        StatusCode::CREATED,
        AuthPayload {
            token: "something".to_string(),
            user: User {
                id: 1,
                username: "something".to_string(),
                password_hash: "something".to_string(),
            },
        },
    ))
}
