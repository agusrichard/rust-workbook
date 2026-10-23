use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};

use crate::{
    AppState,
    auth::{hash_password, validate_password},
    models::{AuthRequest, User},
    response::{ApiResult, AuthPayload, api_error},
};

pub async fn hello_world() -> String {
    "Hello World".to_string()
}

pub async fn register(
    State(state): State<AppState>,
    payload: Result<Json<AuthRequest>, JsonRejection>,
) -> ApiResult<AuthPayload> {
    let Json(payload) =
        payload.map_err(|_| api_error(StatusCode::BAD_REQUEST, "invalid JSON request"))?;

    validate_password(payload.password.as_str())
        .map_err(|_| api_error(StatusCode::BAD_REQUEST, "Invalid password"))?;

    let password = hash_password(&payload.password)
        .map_err(|_| api_error(StatusCode::BAD_REQUEST, "Please try again"))?;

    let pool = state.pool;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, password) VALUES (?, ?) RETURNING *",
    )
    .bind(payload.username)
    .bind(password)
    .fetch_one(&pool)
    .await
    .map_err(|_| api_error(StatusCode::BAD_REQUEST, "Please try again"))?;

    Ok((
        StatusCode::CREATED,
        Json(AuthPayload {
            token: "something".to_string(),
            user: user,
        }),
    ))
}
