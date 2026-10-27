use std::{
    collections::HashMap,
    fmt::Write,
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    Error,
    extract::{Request, State},
    http::{StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite};

use crate::{
    AppState,
    response::{ApiError, api_error},
};

const COOKIE_SECURE_FLAG: bool = false;
const TOKEN_ISSUER: &str = "secure-rest-api";
const TOKEN_AUDIENCE: &str = "secure-rest-api";
const ACCESS_TOKEN_LIFETIME_SECONDS: u64 = 15 * 60;
const REFRESH_TOKEN_LIFETIME_SECONDS: u64 = 30 * 24 * 60 * 60;
const MAX_FAILED_LOGIN_ATTEMPTS: u32 = 5;
const LOGIN_ATTEMPT_WINDOW: Duration = Duration::from_secs(60);
const MAX_TRACKED_LOGIN_IDENTIFIERS: usize = 10_000;

static DUMMY_PASSWORD_HASH: LazyLock<String> = LazyLock::new(|| {
    hash_password("dummy-password-that-is-never-valid")
        .expect("failed to initialize dummy password hash")
});

#[derive(Clone)]
pub struct LoginRateLimiter {
    attempts: Arc<Mutex<HashMap<String, FailedLoginAttempts>>>,
}

struct FailedLoginAttempts {
    count: u32,
    started_at: Instant,
}

impl Default for LoginRateLimiter {
    fn default() -> Self {
        Self {
            attempts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl LoginRateLimiter {
    pub fn is_limited(&self, identifier: &str) -> bool {
        let mut attempts = self
            .attempts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = Instant::now();

        attempts.retain(|_, entry| now.duration_since(entry.started_at) < LOGIN_ATTEMPT_WINDOW);

        if let Some(entry) = attempts.get(identifier) {
            return entry.count >= MAX_FAILED_LOGIN_ATTEMPTS;
        }

        attempts.len() >= MAX_TRACKED_LOGIN_IDENTIFIERS
    }

    pub fn record_failure(&self, identifier: &str) {
        let mut attempts = self
            .attempts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = Instant::now();

        attempts
            .entry(identifier.to_owned())
            .and_modify(|entry| {
                if now.duration_since(entry.started_at) >= LOGIN_ATTEMPT_WINDOW {
                    entry.count = 1;
                    entry.started_at = now;
                } else {
                    entry.count += 1;
                }
            })
            .or_insert(FailedLoginAttempts {
                count: 1,
                started_at: now,
            });
    }

    pub fn reset(&self, identifier: &str) {
        self.attempts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(identifier);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum TokenType {
    Access,
    Refresh,
}

pub fn validate_password(password: &str) -> Result<(), Error> {
    let password_length = password.chars().count();
    if password_length < 15 {
        return Err(Error::new("Password must be at least 15 characters"));
    }

    if password_length > 128 {
        return Err(Error::new("Password must be at most 128 characters"));
    }

    const COMMON_PASSWORDS: &[&str] = &[
        "123456789012345",
        "letmeinletmein",
        "password123456",
        "passwordpassword",
        "qwertyuiopasdfg",
    ];
    let normalized_password = password.trim().to_ascii_lowercase();
    if COMMON_PASSWORDS.contains(&normalized_password.as_str()) {
        return Err(Error::new("Password is too common"));
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

pub fn initialize_dummy_password_hash() {
    LazyLock::force(&DUMMY_PASSWORD_HASH);
}

pub fn verify_dummy_password(password: &str) {
    let _ = verify_password(password, &DUMMY_PASSWORD_HASH);
}

pub fn hash_refresh_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub sid: i64,
    pub iat: u64,
    pub exp: u64,
    pub iss: String,
    pub aud: String,
    pub jti: String,
    pub token_type: TokenType,
}

impl TokenClaims {
    pub fn user_id(&self) -> Option<i64> {
        self.sub.parse().ok()
    }
}

fn generate_jti() -> Result<String, ApiError> {
    let mut random_bytes = [0_u8; 16];
    getrandom::fill(&mut random_bytes).map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Please try again later!",
            Some("Error generating token identifier"),
        )
    })?;

    let mut jti = String::with_capacity(random_bytes.len() * 2);
    for byte in random_bytes {
        write!(&mut jti, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(jti)
}

pub fn issue_token(
    user_id: i64,
    session_id: i64,
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
        sub: user_id.to_string(),
        sid: session_id,
        iat: now,
        exp: now + lifetime_seconds,
        iss: TOKEN_ISSUER.to_owned(),
        aud: TOKEN_AUDIENCE.to_owned(),
        jti: generate_jti()?,
        token_type,
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

pub fn issue_access_token(
    user_id: i64,
    session_id: i64,
    secret: &[u8],
) -> Result<(String, u64), ApiError> {
    issue_token(
        user_id,
        session_id,
        TokenType::Access,
        ACCESS_TOKEN_LIFETIME_SECONDS,
        secret,
    )
}

pub fn issue_refresh_token(
    user_id: i64,
    session_id: i64,
    secret: &[u8],
) -> Result<(String, u64), ApiError> {
    issue_token(
        user_id,
        session_id,
        TokenType::Refresh,
        REFRESH_TOKEN_LIFETIME_SECONDS,
        secret,
    )
}

fn decode_token(
    token: &str,
    secret: &[u8],
    expected_token_type: TokenType,
) -> Result<TokenClaims, ApiError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[TOKEN_ISSUER]);
    validation.set_audience(&[TOKEN_AUDIENCE]);
    validation.set_required_spec_claims(&["exp", "iat", "sub", "iss", "aud", "jti"]);

    let claims = decode::<TokenClaims>(token, &DecodingKey::from_secret(secret), &validation)
        .map_err(|_| api_error(StatusCode::UNAUTHORIZED, "Unauthorized", None))?
        .claims;

    let token_type_matches = matches!(
        (&claims.token_type, expected_token_type),
        (TokenType::Access, TokenType::Access) | (TokenType::Refresh, TokenType::Refresh)
    );
    let jti_is_valid =
        claims.jti.len() == 32 && claims.jti.bytes().all(|byte| byte.is_ascii_hexdigit());

    if !token_type_matches || !jti_is_valid {
        return Err(api_error(StatusCode::UNAUTHORIZED, "Unauthorized", None));
    }

    Ok(claims)
}

pub fn decode_access_token(token: &str, secret: &[u8]) -> Result<TokenClaims, ApiError> {
    decode_token(token, secret, TokenType::Access)
}

pub fn decode_refresh_token(token: &str, secret: &[u8]) -> Result<TokenClaims, ApiError> {
    decode_token(token, secret, TokenType::Refresh)
}

pub async fn is_access_session_active(
    pool: &Pool<Sqlite>,
    claims: &TokenClaims,
) -> Result<bool, ApiError> {
    let Some(user_id) = claims.user_id() else {
        return Ok(false);
    };

    sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM auth_sessions
            WHERE id = ?
              AND user_id = ?
              AND revoked_at IS NULL
              AND expires_at > unixepoch()
        )
        "#,
    )
    .bind(claims.sid)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Please try again later",
            Some("Failed to validate access-token session"),
        )
    })
}

pub async fn require_access_token(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "Unauthorized", None))?;

    let claims = decode_access_token(token, state.access_token_secret.as_bytes())?;
    if !is_access_session_active(&state.pool, &claims).await? {
        return Err(api_error(StatusCode::UNAUTHORIZED, "Unauthorized", None));
    }

    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}

pub fn build_refresh_cookie(token: String) -> Cookie<'static> {
    Cookie::build(("refresh_token", token))
        .http_only(true)
        .secure(COOKIE_SECURE_FLAG)
        .same_site(SameSite::Strict)
        .path("/auth")
        .build()
}

pub async fn login_session(
    pool: &Pool<Sqlite>,
    user_id: i64,
    access_token_secret: &[u8],
    refresh_token_secret: &[u8],
) -> Result<(String, String), ApiError> {
    let mut tx = pool.begin().await.map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Please try again later",
            Some("Error starting transaction"),
        )
    })?;
    let provisional_expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Please try again later!",
                Some("Issue when getting time now()"),
            )
        })?
        .as_secs()
        + REFRESH_TOKEN_LIFETIME_SECONDS;
    let session_id: i64 = sqlx::query_scalar(
        "INSERT INTO auth_sessions (user_id, expires_at) VALUES (?, ?) RETURNING id",
    )
    .bind(user_id)
    .bind(provisional_expires_at as i64)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Please try again later!",
            Some("Error when inserting to auth_sessions"),
        )
    })?;

    let (access_token, _) = issue_access_token(user_id, session_id, access_token_secret)?;
    let (refresh_token, refresh_expires_at) =
        issue_refresh_token(user_id, session_id, refresh_token_secret)?;
    let refresh_token_hash = hash_refresh_token(&refresh_token);

    sqlx::query("UPDATE auth_sessions SET expires_at = ? WHERE id = ?")
        .bind(refresh_expires_at as i64)
        .bind(session_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Please try again later!",
                Some("Error when updating auth_sessions"),
            )
        })?;

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

    Ok((access_token, refresh_token))
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

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"test-signing-secret-with-32-bytes";

    fn encode_claims(claims: &TokenClaims) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(SECRET),
        )
        .unwrap()
    }

    fn valid_access_claims() -> TokenClaims {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        TokenClaims {
            sub: "42".to_owned(),
            sid: 7,
            iat: now,
            exp: now + 60,
            iss: TOKEN_ISSUER.to_owned(),
            aud: TOKEN_AUDIENCE.to_owned(),
            jti: "0123456789abcdef0123456789abcdef".to_owned(),
            token_type: TokenType::Access,
        }
    }

    #[test]
    fn password_policy_checks_length_and_common_passwords() {
        assert!(validate_password("short-password").is_err());
        assert!(validate_password("passwordpassword").is_err());
        assert!(validate_password("correct horse battery staple").is_ok());
        assert!(validate_password(&"a".repeat(129)).is_err());
    }

    #[test]
    fn password_hashes_are_salted_and_verifiable() {
        let first_hash = hash_password("correct horse battery staple").unwrap();
        let second_hash = hash_password("correct horse battery staple").unwrap();

        assert_ne!(first_hash, second_hash);
        assert!(verify_password("correct horse battery staple", &first_hash));
        assert!(!verify_password("wrong password", &first_hash));
    }

    #[test]
    fn tokens_issued_in_the_same_second_are_unique() {
        let (first, _) = issue_refresh_token(42, 7, SECRET).unwrap();
        let (second, _) = issue_refresh_token(42, 7, SECRET).unwrap();

        assert_ne!(first, second);
        let first_claims = decode_refresh_token(&first, SECRET).unwrap();
        let second_claims = decode_refresh_token(&second, SECRET).unwrap();
        assert_ne!(first_claims.jti, second_claims.jti);
    }

    #[test]
    fn access_token_validates_standard_claims_and_type() {
        let (token, _) = issue_access_token(42, 7, SECRET).unwrap();
        let claims = decode_access_token(&token, SECRET).unwrap();

        assert_eq!(claims.user_id(), Some(42));
        assert_eq!(claims.sid, 7);
        assert_eq!(claims.iss, TOKEN_ISSUER);
        assert_eq!(claims.aud, TOKEN_AUDIENCE);
        assert_eq!(claims.jti.len(), 32);
        assert!(decode_refresh_token(&token, SECRET).is_err());
    }

    #[test]
    fn token_rejects_wrong_issuer_audience_or_jti() {
        let mut claims = valid_access_claims();
        claims.iss = "another-issuer".to_owned();
        assert!(decode_access_token(&encode_claims(&claims), SECRET).is_err());

        let mut claims = valid_access_claims();
        claims.aud = "another-audience".to_owned();
        assert!(decode_access_token(&encode_claims(&claims), SECRET).is_err());

        let mut claims = valid_access_claims();
        claims.jti.clear();
        assert!(decode_access_token(&encode_claims(&claims), SECRET).is_err());
    }

    #[test]
    fn login_rate_limiter_blocks_and_can_be_reset() {
        let limiter = LoginRateLimiter::default();
        for _ in 0..MAX_FAILED_LOGIN_ATTEMPTS {
            assert!(!limiter.is_limited("alice"));
            limiter.record_failure("alice");
        }

        assert!(limiter.is_limited("alice"));
        assert!(!limiter.is_limited("bob"));
        limiter.reset("alice");
        assert!(!limiter.is_limited("alice"));
    }
}
