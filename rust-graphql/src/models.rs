use async_graphql::{ComplexObject, SimpleObject, ID, Result, Context, InputObject};
use sqlx::{FromRow, SqlitePool};

#[derive(FromRow, SimpleObject)]
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

    async fn books(&self, ctx: &Context<'_>) -> Result<Vec<Book>> {
        let pool = ctx.data::<SqlitePool>()?;
        let books = sqlx::query_as("SELECT * FROM books WHERE author_id = ? ORDER BY id")
            .bind(self.id)
            .fetch_all(pool)
            .await?;
        Ok(books)
    }
}



#[derive(FromRow, SimpleObject, Clone)]
#[graphql(complex)]
pub struct Book {
    #[graphql(skip)]
    pub id: i64,
    pub title: String,
    pub year: Option<u16>,
    #[graphql(skip)]
    pub author_id: i64
}

#[ComplexObject]
impl Book {
    async fn id(&self) -> ID {
        self.id.into()
    }

    async fn author(&self, ctx: &Context<'_>) -> Result<Author> {
        let pool = ctx.data::<SqlitePool>()?;
        let author = sqlx::query_as::<_, Author>("SELECT * FROM authors WHERE id = ?")
            .bind(self.author_id).fetch_one(pool).await?;
        Ok(author)
    }
}

#[derive(InputObject)]
pub struct CreateBookInput {
    pub title: String,
    pub year: Option<u16>,
    pub author_id: ID
}

#[derive(InputObject)]
pub struct UpdateBookInput {
    pub title: Option<String>,
    pub year: Option<u16>,
    pub author_id: Option<ID>
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
    pub has_next_page: bool
}

#[derive(SimpleObject)]
pub struct AuthorPage {
    pub items: Vec<Author>,
    pub total_count: i64,
    pub has_next_page: bool
}

#[derive(FromRow, SimpleObject)]
#[graphql(complex)]
pub struct User {
    #[graphql(skip)]
    pub id: i64,
    pub username: String,
    #[graphql(skip)]
    pub password_hash: String
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
    pub password: String
}

#[derive(SimpleObject)]
pub struct AuthPayload {
    pub token: String,
    pub user: User
}