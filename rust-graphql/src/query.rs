use async_graphql::{Context, Object, ID};
use sqlx::SqlitePool;
use crate::models::{Author, Book};

pub struct Query;

#[Object]
impl Query {
    async fn authors(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Author>> {
        let pool = ctx.data::<SqlitePool>()?;
        let authors = sqlx::query_as("SELECT * FROM authors ORDER BY id").fetch_all(pool).await?;
        Ok(authors)
    }

    async fn books(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Book>> {
        let pool = ctx.data::<SqlitePool>()?;
        let books = sqlx::query_as("SELECT * FROM books ORDER BY id").fetch_all(pool).await?;
        Ok(books)
    }

    async fn author(&self, ctx: &Context<'_>, id: ID) -> async_graphql::Result<Option<Author>> {
        let id: i64 = id.parse()?;
        let pool = ctx.data::<SqlitePool>()?;
        let author = sqlx::query_as::<_, Author>("SELECT * FROM authors WHERE id = ?").bind(id).fetch_optional(pool).await?;
        Ok(author)
    }

    async fn book(&self, ctx: &Context<'_>, id: ID) -> async_graphql::Result<Option<Book>> {
        let id: i64 = id.parse()?;
        let pool = ctx.data::<SqlitePool>()?;
        let book = sqlx::query_as::<_, Book>("SELECT * FROM books WHERE id = ?").bind(id).fetch_optional(pool).await?;
        Ok(book)
    }
}