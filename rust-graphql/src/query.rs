use async_graphql::{Context, Object, ID, Result, Error};
use sqlx::SqlitePool;
use crate::auth::CurrentUser;
use crate::models::{Author, AuthorPage, Book, BookPage, User};
use crate::validation::validate_page;

pub struct Query;

#[Object]
impl Query {
    async fn authors(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 10)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> Result<AuthorPage> {
        validate_page(limit, offset)?;
        let pool = ctx.data::<SqlitePool>()?;
        let items = sqlx::query_as("
            SELECT * FROM authors
            ORDER BY id
            LIMIT ? OFFSET ?
            ")
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?;

        let total_count = sqlx::query_scalar("SELECT COUNT(*) FROM authors")
            .fetch_one(pool)
            .await?;

        let has_next_page = offset + (items.len() as i64) < total_count;
        Ok(AuthorPage {
            items,
            total_count,
            has_next_page,
        })
    }

    async fn books(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 10)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> Result<BookPage> {
        validate_page(limit, offset)?;
        let pool = ctx.data::<SqlitePool>()?;
        let items = sqlx::query_as("
            SELECT * FROM books
            ORDER BY id
            LIMIT ? OFFSET ?
            ")
            .bind(limit)
            .bind(offset)
            .fetch_all(pool).await?;

        let total_count = sqlx::query_scalar("SELECT COUNT(*) FROM books")
            .fetch_one(pool)
            .await?;

        let has_next_page = offset + (items.len() as i64) < total_count;
        Ok(BookPage {
            items,
            total_count,
            has_next_page,
        })
    }

    async fn author(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Author>> {
        let id: i64 = id.parse()?;
        let pool = ctx.data::<SqlitePool>()?;
        let author = sqlx::query_as::<_, Author>("SELECT * FROM authors WHERE id = ?").bind(id).fetch_optional(pool).await?;
        Ok(author)
    }

    async fn book(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Book>> {
        let id: i64 = id.parse()?;
        let pool = ctx.data::<SqlitePool>()?;
        let book = sqlx::query_as::<_, Book>("SELECT * FROM books WHERE id = ?").bind(id).fetch_optional(pool).await?;
        Ok(book)
    }

    async fn me(&self, ctx: &Context<'_>) -> Result<Option<User>> {
        let Some(current_user) = ctx.data_opt::<CurrentUser>() else { return Ok(None); };

        let pool = ctx.data::<SqlitePool>()?;
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(current_user.id)
            .fetch_optional(pool)
            .await
            .map_err(Error::from)?;

        Ok(user)
    }
}