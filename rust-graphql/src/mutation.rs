use tokio::sync::broadcast::Sender;
use async_graphql::{Context, Object, ID, Error, Result};
use sqlx::SqlitePool;
use crate::auth::LoggedIn;
use crate::auth::{hash_password, make_token, verify_password, JwtSecret};
use crate::models::{AuthPayload, Author, Book, CreateAuthorInput, CreateBookInput, CredentialsInput, UpdateAuthorInput, UpdateBookInput, User};
use crate::validation::{ensure_author_exists, ensure_valid_delete_author, require_text, validate_password};

pub struct Mutation;

#[Object]
impl Mutation {
    #[graphql(guard = "LoggedIn")]
    async fn create_author(&self, ctx: &Context<'_>, input: CreateAuthorInput) -> Result<Author> {
        let name = require_text(&input.name, "Name")?;
        let pool = ctx.data::<SqlitePool>()?;
        let author = sqlx::query_as::<_, Author>("INSERT INTO authors (name) VALUES (?) RETURNING *")
            .bind(name)
            .fetch_one(pool)
            .await?;
        Ok(author)
    }

    #[graphql(guard = "LoggedIn")]
    async fn update_author(&self, ctx: &Context<'_>, id: ID, input: UpdateAuthorInput) -> Result<Author> {
        let id = id.parse::<i64>()?;
        let name = input.name.as_deref().map(|n| require_text(n, "Name")).transpose()?;
        let pool = ctx.data::<SqlitePool>()?;
        let author = sqlx::query_as::<_, Author>("UPDATE authors SET name = COALESCE(?, name) WHERE id = ? RETURNING *")
            .bind(name)
            .bind(id)
            .fetch_optional(pool)
            .await?;
        author.ok_or_else(|| Error::new(format!("Author {id} does not exist")))
    }

    #[graphql(guard = "LoggedIn")]
    async fn delete_author(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let id = id.parse::<i64>()?;
        let pool = ctx.data::<SqlitePool>()?;
        ensure_valid_delete_author(pool, id).await?;
        let result = sqlx::query("DELETE FROM authors WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(Error::new(format!("Author {id} does not exist")));
        }
        Ok(true)
    }

    #[graphql(guard = "LoggedIn")]
    async fn create_book(&self, ctx: &Context<'_>, input: CreateBookInput) -> Result<Book> {
        let title = require_text(&input.title, "Title")?;
        let author_id = input.author_id.parse::<i64>()?;
        let pool = ctx.data::<SqlitePool>()?;
        ensure_author_exists(pool, author_id).await?;
        let book = sqlx::query_as::<_, Book>("INSERT INTO books (title, year, author_id) VALUES (?, ?, ?) RETURNING *")
            .bind(title)
            .bind(input.year)
            .bind(author_id)
            .fetch_one(pool)
            .await?;
        let events = ctx.data::<Sender<Book>>()?;
        let _ = events.send(book.clone());
        Ok(book)
    }

    #[graphql(guard = "LoggedIn")]
    async fn update_book(&self, ctx: &Context<'_>, id: ID, input: UpdateBookInput) -> Result<Book> {
        let id = id.parse::<i64>()?;
        let title = input.title.as_deref().map(|n| require_text(n, "Title")).transpose()?;
        let author_id: Option<i64> = input.author_id.map(|id| id.parse::<i64>()).transpose()?;
        let pool = ctx.data::<SqlitePool>()?;
        if let Some(author_id) = author_id {
            ensure_author_exists(pool, author_id).await?;
        }
        let book = sqlx::query_as::<_, Book>("
                UPDATE books
                SET
                    title = COALESCE(?, title),
                    year = COALESCE(?, year),
                    author_id = COALESCE(?, author_id)
                WHERE id = ?
                RETURNING *")
            .bind(title)
            .bind(input.year)
            .bind(author_id)
            .bind(id)
            .fetch_optional(pool)
            .await?;
        book.ok_or_else(|| Error::new(format!("Book {id} does not exist")))
    }

    #[graphql(guard = "LoggedIn")]
    async fn delete_book(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let id = id.parse::<i64>()?;
        let pool = ctx.data::<SqlitePool>()?;
        let result = sqlx::query("DELETE FROM books WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(Error::new(format!("Book {id} does not exist")))
        }
        Ok(true)
    }

    async fn register(&self, ctx: &Context<'_>, input: CredentialsInput) -> Result<AuthPayload> {
        let username = require_text(&input.username, "Username")?;
        validate_password(input.password.as_str())?;

        let password = hash_password(input.password.as_str())?;
        let pool = ctx.data::<SqlitePool>()?;
        let user = sqlx::query_as::<_, User>("INSERT INTO users (username, password_hash) VALUES (?, ?) RETURNING *")
            .bind(username)
            .bind(password)
            .fetch_one(pool)
            .await
            .map_err(|_| Error::new("Username is already taken"))?;

        let token = make_token(user.id, ctx.data::<JwtSecret>()?)?;

        Ok(AuthPayload {
            user,
            token
        })
    }

    async fn login(&self, ctx: &Context<'_>, input: CredentialsInput) -> Result<AuthPayload> {
        let username = require_text(&input.username, "Username")?;
        let pool = ctx.data::<SqlitePool>()?;
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(pool)
            .await?;
        let user = user.filter(|u| verify_password(&input.password, &u.password_hash))
            .ok_or_else(|| Error::new(format!("Invalid username or password")))?;

        let token = make_token(user.id, ctx.data::<JwtSecret>()?)?;

        Ok(AuthPayload {
            user,
            token
        })
    }
}
