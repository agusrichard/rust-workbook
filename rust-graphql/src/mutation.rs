use async_graphql::{Context, Error, ID, Object, Result};
use sqlx::SqlitePool;
use tokio::sync::broadcast::Sender;

use crate::auth::{CurrentUser, JwtSecret, LoggedIn, hash_password, make_token, verify_password};
use crate::models::{
    AuthPayload, Author, Book, CreateAuthorInput, CreateBookInput, CredentialsInput,
    UpdateAuthorInput, UpdateBookInput, User,
};
use crate::validation::{
    ensure_author_exists, ensure_can_modify_book, ensure_valid_delete_author, require_text,
    validate_password,
};

pub struct Mutation;

#[Object]
impl Mutation {
    #[graphql(guard = "LoggedIn")]
    async fn create_author(&self, ctx: &Context<'_>, input: CreateAuthorInput) -> Result<Author> {
        let name = require_text(&input.name, "Name")?;
        let pool = ctx.data::<SqlitePool>()?;
        let author = sqlx::query_as!(
            Author,
            r#"INSERT INTO authors (name) VALUES (?) RETURNING id AS "id!", name"#,
            name
        )
        .fetch_one(pool)
        .await?;
        Ok(author)
    }

    #[graphql(guard = "LoggedIn")]
    async fn update_author(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateAuthorInput,
    ) -> Result<Author> {
        let id = id.parse::<i64>()?;
        let name = input.name.as_deref().map(|n| require_text(n, "Name")).transpose()?;
        let pool = ctx.data::<SqlitePool>()?;
        let author = sqlx::query_as!(
            Author,
            r#"UPDATE authors SET name = COALESCE(?, name) WHERE id = ?
               RETURNING id AS "id!", name"#,
            name,
            id
        )
        .fetch_optional(pool)
        .await?;
        author.ok_or_else(|| Error::new(format!("Author {id} does not exist")))
    }

    #[graphql(guard = "LoggedIn")]
    async fn delete_author(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let id = id.parse::<i64>()?;
        let pool = ctx.data::<SqlitePool>()?;
        ensure_valid_delete_author(pool, id).await?;
        let result = sqlx::query!("DELETE FROM authors WHERE id = ?", id)
            .execute(pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(Error::new(format!("Author {id} does not exist")));
        }
        Ok(true)
    }

    // The logged-in user becomes the book's owner.
    #[graphql(guard = "LoggedIn")]
    async fn create_book(&self, ctx: &Context<'_>, input: CreateBookInput) -> Result<Book> {
        let title = require_text(&input.title, "Title")?;
        let year = input.year.map(i64::from);
        let author_id = input.author_id.parse::<i64>()?;
        let user = ctx.data::<CurrentUser>()?;
        let pool = ctx.data::<SqlitePool>()?;
        ensure_author_exists(pool, author_id).await?;
        let book = sqlx::query_as!(
            Book,
            r#"INSERT INTO books (title, year, author_id, created_by) VALUES (?, ?, ?, ?)
               RETURNING id AS "id!", title, year, author_id, created_by"#,
            title,
            year,
            author_id,
            user.id
        )
        .fetch_one(pool)
        .await?;
        let events = ctx.data::<Sender<Book>>()?;
        let _ = events.send(book.clone());
        Ok(book)
    }

    // Only the book's owner may change it. Seed books have no owner, so any
    // logged-in user may change those.
    #[graphql(guard = "LoggedIn")]
    async fn update_book(&self, ctx: &Context<'_>, id: ID, input: UpdateBookInput) -> Result<Book> {
        let id = id.parse::<i64>()?;
        let title = input.title.as_deref().map(|n| require_text(n, "Title")).transpose()?;
        let year = input.year.map(i64::from);
        let author_id: Option<i64> = input.author_id.map(|id| id.parse::<i64>()).transpose()?;
        let user = ctx.data::<CurrentUser>()?;
        let pool = ctx.data::<SqlitePool>()?;
        ensure_can_modify_book(pool, id, user).await?;
        if let Some(author_id) = author_id {
            ensure_author_exists(pool, author_id).await?;
        }
        let book = sqlx::query_as!(
            Book,
            r#"UPDATE books
               SET
                   title = COALESCE(?, title),
                   year = COALESCE(?, year),
                   author_id = COALESCE(?, author_id)
               WHERE id = ?
               RETURNING id AS "id!", title, year, author_id, created_by"#,
            title,
            year,
            author_id,
            id
        )
        .fetch_optional(pool)
        .await?;
        book.ok_or_else(|| Error::new(format!("Book {id} does not exist")))
    }

    #[graphql(guard = "LoggedIn")]
    async fn delete_book(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let id = id.parse::<i64>()?;
        let user = ctx.data::<CurrentUser>()?;
        let pool = ctx.data::<SqlitePool>()?;
        ensure_can_modify_book(pool, id, user).await?;
        let result = sqlx::query!("DELETE FROM books WHERE id = ?", id)
            .execute(pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(Error::new(format!("Book {id} does not exist")));
        }
        Ok(true)
    }

    async fn register(&self, ctx: &Context<'_>, input: CredentialsInput) -> Result<AuthPayload> {
        let username = require_text(&input.username, "Username")?;
        validate_password(input.password.as_str())?;

        let password = hash_password(input.password.as_str())?;
        let pool = ctx.data::<SqlitePool>()?;
        let user = sqlx::query_as!(
            User,
            r#"INSERT INTO users (username, password_hash) VALUES (?, ?)
               RETURNING id AS "id!", username, password_hash"#,
            username,
            password
        )
        .fetch_one(pool)
        .await
        .map_err(|_| Error::new("Username is already taken"))?;

        let token = make_token(user.id, ctx.data::<JwtSecret>()?)?;

        Ok(AuthPayload { user, token })
    }

    async fn login(&self, ctx: &Context<'_>, input: CredentialsInput) -> Result<AuthPayload> {
        let username = require_text(&input.username, "Username")?;
        let pool = ctx.data::<SqlitePool>()?;
        let user = sqlx::query_as!(
            User,
            r#"SELECT id AS "id!", username, password_hash FROM users WHERE username = ?"#,
            username
        )
        .fetch_optional(pool)
        .await?;
        let user = user
            .filter(|u| verify_password(&input.password, &u.password_hash))
            .ok_or_else(|| Error::new("Invalid username or password"))?;

        let token = make_token(user.id, ctx.data::<JwtSecret>()?)?;

        Ok(AuthPayload { user, token })
    }
}
