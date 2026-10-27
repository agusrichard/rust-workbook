use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
};

use axum_extra::extract::cookie::CookieJar;

use crate::{
    AppState,
    auth::{
        build_refresh_cookie, build_refresh_cookie_removal, decode_access_token,
        decode_refresh_token, hash_password, hash_refresh_token, issue_access_token,
        issue_refresh_token, login_session, validate_password, verify_dummy_password,
        verify_password,
    },
    models::{AuthPayload, AuthRequest, User},
    response::{ApiResult, api_error, no_store_headers},
};

pub async fn hello_world() -> String {
    "Hello World".to_string()
}

pub async fn me() -> String {
    "This is an amazing content bro!".to_string()
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
            "Unable to register with the provided credentials",
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

    let (access_token, refresh_token) = login_session(
        &pool,
        user.id,
        state.access_token_secret.as_bytes(),
        state.refresh_token_secret.as_bytes(),
    )
    .await?;

    let jar = jar.add(build_refresh_cookie(refresh_token));

    Ok((
        StatusCode::CREATED,
        no_store_headers(),
        jar,
        Json(AuthPayload {
            access_token,
            user: Some(user),
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    payload: Result<Json<AuthRequest>, JsonRejection>,
) -> ApiResult<AuthPayload> {
    let Json(payload) =
        payload.map_err(|_| api_error(StatusCode::BAD_REQUEST, "invalid JSON request", None))?;

    if state.login_rate_limiter.is_limited(&payload.username) {
        return Err(api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many login attempts. Try again later",
            None,
        ));
    }

    let pool = state.pool;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(&payload.username)
        .fetch_optional(&pool)
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Please try to login again later!",
                Some("Error get user data"),
            )
        })?;

    let password_is_valid = match &user {
        Some(user) => verify_password(&payload.password, &user.password_hash),
        None => {
            verify_dummy_password(&payload.password);
            false
        }
    };

    if !password_is_valid {
        state.login_rate_limiter.record_failure(&payload.username);
        return Err(api_error(
            StatusCode::UNAUTHORIZED,
            "Invalid username or password",
            None,
        ));
    }
    state.login_rate_limiter.reset(&payload.username);
    let user = user.expect("a valid password always has a matching user");

    let (access_token, refresh_token) = login_session(
        &pool,
        user.id,
        state.access_token_secret.as_bytes(),
        state.refresh_token_secret.as_bytes(),
    )
    .await?;

    let jar = jar.add(build_refresh_cookie(refresh_token));

    Ok((
        StatusCode::OK,
        no_store_headers(),
        jar,
        Json(AuthPayload {
            access_token,
            user: Some(user),
        }),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
) -> ApiResult<String> {
    let access_session_id = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(|token| {
            decode_access_token(token, state.access_token_secret.as_bytes())
                .ok()
                .map(|claims| claims.sid)
        });

    if let Some(session_id) = access_session_id {
        sqlx::query(
            "UPDATE auth_sessions SET revoked_at = COALESCE(revoked_at, unixepoch()) WHERE id = ?",
        )
        .bind(session_id)
        .execute(&state.pool)
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Please try again later",
                Some("Failed to revoke access-token session"),
            )
        })?;
    }

    if let Some(refresh_token) = jar
        .get("refresh_token")
        .map(|cookie| cookie.value().to_owned())
    {
        let token_hash = hash_refresh_token(&refresh_token);

        sqlx::query(
            r#"
            UPDATE auth_sessions
            SET revoked_at = unixepoch()
            WHERE id = (
                SELECT session_id
                FROM refresh_tokens
                WHERE token_hash = ?
            )
            AND revoked_at IS NULL
            "#,
        )
        .bind(token_hash.as_slice())
        .execute(&state.pool)
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Please try again later",
                Some("Failed to revoke authentication session"),
            )
        })?;
    }

    let jar = jar.add(build_refresh_cookie_removal());

    Ok((
        StatusCode::OK,
        no_store_headers(),
        jar,
        Json("Logout success".to_string()),
    ))
}

pub async fn refresh(State(state): State<AppState>, jar: CookieJar) -> ApiResult<AuthPayload> {
    let invalid_token = || api_error(StatusCode::UNAUTHORIZED, "Invalid refresh token", None);

    let Some(refresh_token) = jar
        .get("refresh_token")
        .map(|cookie| cookie.value().to_owned())
    else {
        return Err(invalid_token());
    };
    let refresh_claims =
        decode_refresh_token(&refresh_token, state.refresh_token_secret.as_bytes())
            .map_err(|_| invalid_token())?;

    let token_hash = hash_refresh_token(&refresh_token);
    let mut tx = state.pool.begin().await.map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Please try again later",
            Some("Error starting transaction"),
        )
    })?;

    let session_id: Option<i64> = sqlx::query_scalar(
        r#"
        UPDATE refresh_tokens
        SET consumed_at = unixepoch()
        WHERE token_hash = ? AND consumed_at IS NULL
        RETURNING session_id
        "#,
    )
    .bind(token_hash.as_slice())
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Please try again later",
            Some("Failed to consume refresh token"),
        )
    })?;

    let Some(session_id) = session_id else {
        let reused_session_id: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT session_id
            FROM refresh_tokens
            WHERE token_hash = ?
            "#,
        )
        .bind(token_hash.as_slice())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Please try again later",
                Some("Failed to check refresh-token reuse"),
            )
        })?;

        if let Some(session_id) = reused_session_id {
            sqlx::query(
                "UPDATE auth_sessions SET revoked_at = COALESCE(revoked_at, unixepoch()) WHERE id = ?",
            )
            .bind(session_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Please try again later",
                    Some("Failed to revoke reused refresh-token session"),
                )
            })?;

            tx.commit().await.map_err(|_| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Please try again later",
                    Some("Failed to commit refresh-token reuse revocation"),
                )
            })?;
        }

        return Err(invalid_token());
    };

    if refresh_claims.sid != session_id {
        return Err(invalid_token());
    }
    let refresh_user_id = refresh_claims.user_id().ok_or_else(invalid_token)?;

    let user_id: i64 = sqlx::query_scalar(
        r#"
        SELECT user_id
        FROM auth_sessions
        WHERE id = ?
          AND user_id = ?
          AND revoked_at IS NULL
          AND expires_at > unixepoch()
        "#,
    )
    .bind(session_id)
    .bind(refresh_user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Please try again later",
            Some("Failed to validate refresh session"),
        )
    })?
    .ok_or_else(invalid_token)?;

    let (access_token, _) =
        issue_access_token(user_id, session_id, state.access_token_secret.as_bytes())?;
    let (new_refresh_token, _) =
        issue_refresh_token(user_id, session_id, state.refresh_token_secret.as_bytes())?;

    let refresh_token_hash = hash_refresh_token(&new_refresh_token);
    sqlx::query("INSERT INTO refresh_tokens (session_id, token_hash) VALUES (?, ?)")
        .bind(session_id)
        .bind(refresh_token_hash.as_slice())
        .execute(&mut *tx)
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Please try again later!",
                Some("Error when inserting to refresh_tokens"),
            )
        })?;

    tx.commit().await.map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Please try again later!",
            Some("Error commiting the transaction"),
        )
    })?;

    let jar = jar.add(build_refresh_cookie(new_refresh_token));

    Ok((
        StatusCode::OK,
        no_store_headers(),
        jar,
        Json(AuthPayload {
            access_token,
            user: None,
        }),
    ))
}
