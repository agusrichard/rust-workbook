use async_graphql::{Error, Result};
use sqlx::SqlitePool;

use crate::auth::CurrentUser;

pub fn require_text(value: &str, field: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::new(format!("{field} cannot be blank")));
    }

    Ok(trimmed.to_string())
}

pub async fn ensure_author_exists(pool: &SqlitePool, id: i64) -> Result<()> {
    let found = sqlx::query_scalar!("SELECT id FROM authors WHERE id = ?", id)
        .fetch_optional(pool)
        .await?;
    if found.is_none() {
        return Err(Error::new(format!("Author {id} does not exist")));
    }
    Ok(())
}

pub async fn ensure_valid_delete_author(pool: &SqlitePool, author_id: i64) -> Result<()> {
    let found = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM books WHERE author_id = ?"#,
        author_id
    )
    .fetch_one(pool)
    .await?;
    if found > 0 {
        return Err(Error::new(format!(
            "Author {author_id} has books and cannot be deleted"
        )));
    }

    Ok(())
}

// A book may be changed by the user who created it. Books without an owner
// (the seed data) may be changed by any logged-in user.
pub async fn ensure_can_modify_book(pool: &SqlitePool, book_id: i64, user: &CurrentUser) -> Result<()> {
    let book = sqlx::query!("SELECT created_by FROM books WHERE id = ?", book_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::new(format!("Book {book_id} does not exist")))?;

    match book.created_by {
        Some(owner) if owner != user.id => {
            Err(Error::new(format!("Book {book_id} belongs to another user")))
        }
        _ => Ok(()),
    }
}

pub fn validate_page(limit: i64, offset: i64) -> Result<()> {
    if !(1..=50).contains(&limit) {
        return Err(Error::new("Limit should be between 1 and 50"));
    }
    if offset < 0 {
        return Err(Error::new("Offset cannot be negative"));
    }
    Ok(())
}

pub fn validate_password(password: &str) -> Result<()> {
    if password.len() < 8 {
        return Err(Error::new("Password must be at least 8 characters"));
    }

    Ok(())
}
