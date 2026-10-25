use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(FromRow, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,

    #[serde(skip_serializing)]
    pub password_hash: String,
}

#[derive(Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthPayload {
    pub token: String,
    pub user: User,
}
