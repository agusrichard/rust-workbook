use std::time::{Duration, SystemTime, UNIX_EPOCH};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use async_graphql::{Context, Guard, Result};
use axum::http::header::AUTHORIZATION;
use axum::http::HeaderMap;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};


#[derive(Clone)]
pub struct JwtSecret(pub String);

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: i64,   // the user id
    exp: usize, // expiry, in seconds since 1970
}

pub struct CurrentUser {
    pub id: i64
}

pub fn hash_password(password: &str) -> Result<String> {
    let hash = Argon2::default().hash_password(password.as_bytes())?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    match PasswordHash::new(stored_hash) {
        Ok(hash) => Argon2::default().verify_password(password.as_bytes(),
                                                      &hash).is_ok(),
        Err(_) => false,
    }
}

pub fn make_token(user_id: i64, secret: &JwtSecret) -> Result<String> {
    let expires = SystemTime::now() + Duration::from_secs(24 * 60 * 60);
    let claims = Claims {
        sub: user_id,
        exp: expires.duration_since(UNIX_EPOCH)?.as_secs() as usize,
    };
    let key = EncodingKey::from_secret(secret.0.as_bytes());
    Ok(jsonwebtoken::encode(&Header::default(), &claims, &key)?)
}

pub fn user_from_headers(headers: &HeaderMap, secret: &JwtSecret) -> Option<CurrentUser> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    let key = DecodingKey::from_secret(secret.0.as_bytes());
    let data = jsonwebtoken::decode::<Claims>(token, &key, &Validation::default()).ok()?;
    Some(CurrentUser { id: data.claims.sub })
}

pub struct LoggedIn;

impl Guard for LoggedIn {
    async fn check(&self, ctx: &Context<'_>) -> Result<()> {
        match ctx.data_opt::<CurrentUser>() {
            Some(_) => Ok(()),
            None => Err("Not authenticated".into()),
        }
    }
}