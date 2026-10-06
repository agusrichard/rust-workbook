use async_graphql::{Context, ID, Object, Result};
use sqlx::SqlitePool;

use crate::auth::CurrentUser;
use crate::models::{Author, AuthorPage, Book, BookPage, User};
use crate::pagination::{self, AuthorConnection, BookConnection, BookFilter};
use crate::validation::validate_page;

pub struct Query;

#[Object]
impl Query {
    // ---- Offset pagination ----

    #[graphql(complexity = "limit.clamp(1, 50) as usize * child_complexity")]
    async fn authors(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 10)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> Result<AuthorPage> {
        validate_page(limit, offset)?;
        let pool = ctx.data::<SqlitePool>()?;
        let items = sqlx::query_as!(
            Author,
            "SELECT id, name FROM authors ORDER BY id LIMIT ? OFFSET ?",
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let total_count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM authors"#)
            .fetch_one(pool)
            .await?;

        let has_next_page = offset + (items.len() as i64) < total_count;
        Ok(AuthorPage {
            items,
            total_count,
            has_next_page,
        })
    }

    // `authorId` and `titleContains` are optional filters. `totalCount` counts
    // the rows that match them.
    #[graphql(complexity = "limit.clamp(1, 50) as usize * child_complexity")]
    async fn books(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 10)] limit: i64,
        #[graphql(default = 0)] offset: i64,
        author_id: Option<ID>,
        title_contains: Option<String>,
    ) -> Result<BookPage> {
        validate_page(limit, offset)?;
        let filter = BookFilter::new(author_id, title_contains)?;
        let pool = ctx.data::<SqlitePool>()?;

        // `?1 IS NULL OR ...` switches a condition off when its filter is not given.
        let items = sqlx::query_as!(
            Book,
            "SELECT id, title, year, author_id, created_by FROM books
             WHERE (?1 IS NULL OR author_id = ?1)
               AND (?2 IS NULL OR instr(lower(title), lower(?2)) > 0)
             ORDER BY id
             LIMIT ?3 OFFSET ?4",
            filter.author_id,
            filter.title_contains,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let total_count = pagination::count_books(pool, &filter).await?;

        let has_next_page = offset + (items.len() as i64) < total_count;
        Ok(BookPage {
            items,
            total_count,
            has_next_page,
        })
    }

    // ---- Cursor pagination ----

    // Forward: `first` and `after`. Backward: `last` and `before`.
    #[graphql(complexity = "pagination::page_cost(first, last) * child_complexity")]
    async fn authors_connection(
        &self,
        ctx: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<AuthorConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        pagination::author_connection(pool, after, before, first, last).await
    }

    #[graphql(complexity = "pagination::page_cost(first, last) * child_complexity")]
    async fn books_connection(
        &self,
        ctx: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
        author_id: Option<ID>,
        title_contains: Option<String>,
    ) -> Result<BookConnection> {
        let filter = BookFilter::new(author_id, title_contains)?;
        let pool = ctx.data::<SqlitePool>()?;
        pagination::book_connection(pool, filter, after, before, first, last).await
    }

    // ---- Single items ----

    async fn author(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Author>> {
        let id: i64 = id.parse()?;
        let pool = ctx.data::<SqlitePool>()?;
        let author = sqlx::query_as!(Author, "SELECT id, name FROM authors WHERE id = ?", id)
            .fetch_optional(pool)
            .await?;
        Ok(author)
    }

    async fn book(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Book>> {
        let id: i64 = id.parse()?;
        let pool = ctx.data::<SqlitePool>()?;
        let book = sqlx::query_as!(
            Book,
            "SELECT id, title, year, author_id, created_by FROM books WHERE id = ?",
            id
        )
        .fetch_optional(pool)
        .await?;
        Ok(book)
    }

    async fn me(&self, ctx: &Context<'_>) -> Result<Option<User>> {
        let Some(current_user) = ctx.data_opt::<CurrentUser>() else {
            return Ok(None);
        };

        let pool = ctx.data::<SqlitePool>()?;
        let user = sqlx::query_as!(
            User,
            "SELECT id, username, password_hash FROM users WHERE id = ?",
            current_user.id
        )
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }
}
