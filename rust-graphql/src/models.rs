use async_graphql::{ComplexObject, SimpleObject, ID, Result, Context};
use sqlx::SqlitePool;

#[derive(sqlx::FromRow, SimpleObject)]
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



#[derive(sqlx::FromRow, SimpleObject)]
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