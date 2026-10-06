use async_graphql::dataloader::DataLoader;
use async_graphql::{ComplexObject, Context, Error, ID, InputObject, Result, SimpleObject};
use sqlx::{FromRow, SqlitePool};

use crate::loaders::{AuthorLoader, BooksByAuthorLoader};
use crate::pagination::{self, BookConnection, BookFilter};

#[derive(FromRow, SimpleObject, Clone)]
#[graphql(complex)]
pub struct Author {
    #[graphql(skip)]
    pub id: i64,
    pub name: String,
}

#[ComplexObject]
impl Author {
    async fn id(&self) -> ID {
        self.id.into()
    }

    // All of the author's books. Batched: one query for every author in the request.
    async fn books(&self, ctx: &Context<'_>) -> Result<Vec<Book>> {
        let loader = ctx.data::<DataLoader<BooksByAuthorLoader>>()?;
        let books = loader.load_one(self.id).await?;
        Ok(books.unwrap_or_default())
    }

    // The author's books, one page at a time.
    #[graphql(complexity = "pagination::page_cost(first, last) * child_complexity")]
    async fn books_connection(
        &self,
        ctx: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<BookConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let filter = BookFilter {
            author_id: Some(self.id),
            title_contains: None,
        };
        pagination::book_connection(pool, filter, after, before, first, last).await
    }
}

#[derive(FromRow, SimpleObject, Clone)]
#[graphql(complex)]
pub struct Book {
    #[graphql(skip)]
    pub id: i64,
    pub title: String,
    pub year: Option<i64>,
    #[graphql(skip)]
    pub author_id: i64,
    // The user who created the book. None for the seed books.
    #[graphql(skip)]
    pub created_by: Option<i64>,
}

#[ComplexObject]
impl Book {
    async fn id(&self) -> ID {
        self.id.into()
    }

    // Batched: one query for every book in the request.
    async fn author(&self, ctx: &Context<'_>) -> Result<Author> {
        let loader = ctx.data::<DataLoader<AuthorLoader>>()?;
        let author = loader.load_one(self.author_id).await?;
        author.ok_or_else(|| Error::new(format!("Author {} does not exist", self.author_id)))
    }

    async fn created_by(&self, ctx: &Context<'_>) -> Result<Option<User>> {
        let Some(user_id) = self.created_by else {
            return Ok(None);
        };
        let pool = ctx.data::<SqlitePool>()?;
        let user = sqlx::query_as!(
            User,
            "SELECT id, username, password_hash FROM users WHERE id = ?",
            user_id
        )
        .fetch_optional(pool)
        .await?;
        Ok(user)
    }
}

#[derive(InputObject)]
pub struct CreateBookInput {
    pub title: String,
    pub year: Option<u16>,
    pub author_id: ID,
}

#[derive(InputObject)]
pub struct UpdateBookInput {
    pub title: Option<String>,
    pub year: Option<u16>,
    pub author_id: Option<ID>,
}

#[derive(InputObject)]
pub struct CreateAuthorInput {
    pub name: String,
}

#[derive(InputObject)]
pub struct UpdateAuthorInput {
    pub name: Option<String>,
}

#[derive(SimpleObject)]
pub struct BookPage {
    pub items: Vec<Book>,
    pub total_count: i64,
    pub has_next_page: bool,
}

#[derive(SimpleObject)]
pub struct AuthorPage {
    pub items: Vec<Author>,
    pub total_count: i64,
    pub has_next_page: bool,
}

#[derive(FromRow, SimpleObject)]
#[graphql(complex)]
pub struct User {
    #[graphql(skip)]
    pub id: i64,
    pub username: String,
    #[graphql(skip)]
    pub password_hash: String,
}

#[ComplexObject]
impl User {
    async fn id(&self) -> ID {
        self.id.into()
    }
}

#[derive(InputObject)]
pub struct CredentialsInput {
    pub username: String,
    pub password: String,
}

#[derive(SimpleObject)]
pub struct AuthPayload {
    pub token: String,
    pub user: User,
}
