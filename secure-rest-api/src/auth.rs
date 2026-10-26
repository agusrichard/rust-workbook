use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{Error, http::StatusCode};
use axum_extra::extract::cookie::{Cookie, SameSite};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

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
) -> Result<String, ApiError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Please try again later!"))?
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
    .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Please try again later!"))?;

    Ok(token)
}

pub fn issue_access_token(user_id: i64, secret: &[u8]) -> Result<String, ApiError> {
    issue_token(user_id, TokenType::Access, 15 * 60, secret)
}

pub fn issue_refresh_token(user_id: i64, secret: &[u8]) -> Result<String, ApiError> {
    issue_token(user_id, TokenType::Refresh, 30 * 24 * 60 * 60, secret)
}

pub fn build_refresh_cookie(token: String) -> Cookie<'static> {
    Cookie::build(("refresh_token", token))
        .http_only(true)
        .secure(COOKIE_SECURE_FLAG)
        .same_site(SameSite::Strict)
        .path("/auth/refresh")
        .build()
}
