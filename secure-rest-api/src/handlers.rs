use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};

use crate::{
    AppState,
    auth::{hash_password, validate_password},
    models::{AuthPayload, AuthRequest, User},
    response::{ApiResult, api_error},
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
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, &e.to_string()))?;

    let password = hash_password(&payload.password)
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, &e.to_string()))?;

    let pool = state.pool;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, password_hash) VALUES (?, ?) RETURNING *",
    )
    .bind(payload.username)
    .bind(password)
    .fetch_one(&pool)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(ref db_error) if db_error.is_unique_violation() => {
            api_error(StatusCode::BAD_REQUEST, "Username is already registered")
        }
        sqlx::Error::PoolTimedOut => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Database is temporarily unavailable",
        ),

        sqlx::Error::Io(_) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Could not connect to the database",
        ),
        unexpected => {
            // Log internal details, but don't expose them to the client.
            eprintln!("Registration database error: {unexpected}");

            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Please try again later")
        }
    })?;

    Ok((
        StatusCode::CREATED,
        Json(AuthPayload {
            token: "something".to_string(),
            user: user,
        }),
    ))
}
