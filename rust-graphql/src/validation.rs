use async_graphql::{Result, Error};
use sqlx::SqlitePool;

pub fn require_text(value: &str, field: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::new(format!("{field} cannot be blank")));
    }

    Ok(trimmed.to_string())
}

pub async fn ensure_author_exists(pool: &SqlitePool, id: i64) -> Result<()> {
    let found: Option<i64> = sqlx::query_scalar("SELECT id FROM authors WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    if found.is_none() {
        return Err(Error::new(format!("Author {id} does not exist")));
    }
    Ok(())
}

pub async fn ensure_valid_delete_author(pool: &SqlitePool, author_id: i64) -> Result<()> {
    let found: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM books WHERE author_id = ?")
        .bind(author_id)
        .fetch_one(pool)
        .await?;
    if found > 0 {
        return Err(Error::new(format!("Author {author_id} has books and cannot be deleted")))
    }

    Ok(())
}

pub fn validate_page(limit: i64, offset: i64) -> Result<()> {
    if !(1..=50).contains(&limit) {
        return Err(Error::new(format!("Limit should be between 1 and 50")))
    }
    if offset < 0 {
        return Err(Error::new(format!("Offset cannot be negative")))
    }
    Ok(())
}

pub fn validate_password(password: &str) -> Result<()> {
    if password.len() < 8 {
        return Err(Error::new(format!("Password must be at least 8 characters")))
    }

    Ok(())
}