use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};

use axum_extra::extract::cookie::CookieJar;

use crate::{
    AppState,
    auth::{
        build_refresh_cookie, hash_password, issue_access_token, issue_refresh_token, user_login,
        validate_password, verify_password,
    },
    models::{AuthPayload, AuthRequest, User},
    response::{ApiResult, api_error},
};

pub async fn hello_world() -> String {
    "Hello World".to_string()
}

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    payload: Result<Json<AuthRequest>, JsonRejection>,
) -> ApiResult<AuthPayload> {
    let Json(payload) =
        payload.map_err(|_| api_error(StatusCode::BAD_REQUEST, "invalid JSON request", None))?;

    validate_password(payload.password.as_str()).map_err(|e| {
        api_error(
            StatusCode::BAD_REQUEST,
            &e.to_string(),
            Some(&e.to_string()),
        )
    })?;

    let password = hash_password(&payload.password).map_err(|e| {
        api_error(
            StatusCode::BAD_REQUEST,
            &e.to_string(),
            Some(&e.to_string()),
        )
    })?;

    let pool = state.pool;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, password_hash) VALUES (?, ?) RETURNING *",
    )
    .bind(payload.username)
    .bind(password)
    .fetch_one(&pool)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(ref db_error) if db_error.is_unique_violation() => api_error(
            StatusCode::BAD_REQUEST,
            "Username is already registered",
            None,
        ),
        sqlx::Error::PoolTimedOut => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Database is temporarily unavailable",
            None,
        ),

        sqlx::Error::Io(_) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Could not connect to the database",
            None,
        ),
        _ => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Please try again later",
            None,
        ),
    })?;

    let (access_token, _) = issue_access_token(user.id, state.access_token_secret.as_bytes())?;
    let (refresh_token, refresh_expires_at) =
        issue_refresh_token(user.id, state.refresh_token_secret.as_bytes())?;

    user_login(&pool, user.id, refresh_expires_at as i64, &refresh_token).await?;

    let jar = jar.add(build_refresh_cookie(refresh_token));

    Ok((
        StatusCode::CREATED,
        jar,
        Json(AuthPayload { access_token, user }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    payload: Result<Json<AuthRequest>, JsonRejection>,
) -> ApiResult<AuthPayload> {
    let Json(payload) =
        payload.map_err(|_| api_error(StatusCode::BAD_REQUEST, "invalid JSON request", None))?;

    let pool = state.pool;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(payload.username)
        .fetch_optional(&pool)
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Please try to login again later!",
                Some("Error get user data"),
            )
        })?
        .ok_or_else(|| {
            api_error(
                StatusCode::BAD_REQUEST,
                "Invalid username or password",
                None,
            )
        })?;

    if !verify_password(&payload.password, &user.password_hash) {
        return Err(api_error(
            StatusCode::UNAUTHORIZED,
            "Invalid username or password",
            None,
        ));
    }

    let (access_token, _) = issue_access_token(user.id, state.access_token_secret.as_bytes())?;
    let (refresh_token, refresh_expires_at) =
        issue_refresh_token(user.id, state.refresh_token_secret.as_bytes())?;

    user_login(&pool, user.id, refresh_expires_at as i64, &refresh_token).await?;

    let jar = jar.add(build_refresh_cookie(refresh_token));

    Ok((
        StatusCode::OK,
        jar,
        Json(AuthPayload { access_token, user }),
    ))
}
