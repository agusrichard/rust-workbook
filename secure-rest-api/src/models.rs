use sqlx::prelude::FromRow;

#[derive(FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

pub struct AuthRequest {
    pub username: String,
    pub password: String,
}
