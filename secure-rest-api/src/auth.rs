use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{Error, http::StatusCode};
use axum_extra::extract::cookie::{Cookie, SameSite};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite};

use crate::response::{ApiError, api_error};

const COOKIE_SECURE_FLAG: bool = false;

#[derive(Serialize, Deserialize)]
pub enum TokenType {
    Access,
    Refresh,
}

pub fn validate_password(password: &str) -> Result<(), Error> {
    if password.len() < 8 {
        return Err(Error::new("Password must be at least 8 characters"));
    }

    Ok(())
}

pub fn hash_password(password: &str) -> Result<String, Error> {
    let hash = Argon2::default()
        .hash_password(password.as_bytes())
        .map_err(|_| Error::new("Please try again"))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    match PasswordHash::new(stored_hash) {
        Ok(hash) => Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok(),
        Err(_) => false,
    }
}

pub fn hash_refresh_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

#[derive(Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: i64,
    pub iat: u64,
    pub exp: u64,
    pub token_type: TokenType,
}

pub fn issue_token(
    user_id: i64,
    token_type: TokenType,
    lifetime_seconds: u64,
    secret: &[u8],
) -> Result<(String, u64), ApiError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Please try again later!",
                Some("Issue when getting time now()"),
            )
        })?
        .as_secs();

    let claims = TokenClaims {
        sub: user_id,
        iat: now,
        exp: now + lifetime_seconds,
        token_type: token_type,
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Please try again later!",
            Some("Error when encoding token"),
        )
    })?;

    Ok((token, now + lifetime_seconds))
}

pub fn issue_access_token(user_id: i64, secret: &[u8]) -> Result<(String, u64), ApiError> {
    issue_token(user_id, TokenType::Access, 15 * 60, secret)
}

pub fn issue_refresh_token(user_id: i64, secret: &[u8]) -> Result<(String, u64), ApiError> {
    issue_token(user_id, TokenType::Refresh, 30 * 24 * 60 * 60, secret)
}

pub fn build_refresh_cookie(token: String) -> Cookie<'static> {
    Cookie::build(("refresh_token", token))
        .http_only(true)
        .secure(COOKIE_SECURE_FLAG)
        .same_site(SameSite::Strict)
        .path("/auth")
        .build()
}

pub async fn user_login(
    pool: &Pool<Sqlite>,
    user_id: i64,
    session_expires_at: i64,
    refresh_token: &str,
) -> Result<(), ApiError> {
    let refresh_token = hash_refresh_token(refresh_token);

    let mut tx = pool.begin().await.map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Please try again later",
            Some("Error starting transaction"),
        )
    })?;
    let session_id: i64 = sqlx::query_scalar(
        "INSERT INTO auth_sessions (user_id, expires_at) VALUES (?, ?) RETURNING id",
    )
    .bind(user_id)
    .bind(session_expires_at)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Please try again later!",
            Some("Error when inserting to auth_sessions"),
        )
    })?;

    sqlx::query("INSERT INTO refresh_tokens (session_id, token_hash) VALUES (?, ?)")
        .bind(session_id)
        .bind(refresh_token.as_slice())
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            eprintln!("{}", e);
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

    Ok(())
}

pub fn build_refresh_cookie_removal() -> Cookie<'static> {
    let mut cookie = Cookie::build("refresh_token")
        .http_only(true)
        .secure(COOKIE_SECURE_FLAG)
        .same_site(SameSite::Strict)
        .path("/auth")
        .build();

    cookie.make_removal();
    cookie
}
