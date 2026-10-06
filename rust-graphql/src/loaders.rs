use std::collections::HashMap;
use std::sync::Arc;

use async_graphql::dataloader::Loader;
use sqlx::SqlitePool;

use crate::models::{Author, Book};

// A loader collects the keys that many resolvers ask for during one request
// and fetches them with a single query, instead of one query per resolver.

// SQLite cannot bind a list to `IN (?)`, so the ids travel as one JSON array
// and json_each() turns it back into rows.
fn json_ids(ids: &[i64]) -> String {
    let ids: Vec<String> = ids.iter().map(i64::to_string).collect();
    format!("[{}]", ids.join(","))
}

// Author by id. Used by `Book.author`.
pub struct AuthorLoader(pub SqlitePool);

impl Loader<i64> for AuthorLoader {
    type Value = Author;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, ids: &[i64]) -> Result<HashMap<i64, Author>, Self::Error> {
        let ids = json_ids(ids);
        let authors = sqlx::query_as!(
            Author,
            // "id!" tells sqlx the column is never NULL; it cannot work that out here.
            r#"SELECT id AS "id!", name FROM authors
               WHERE id IN (SELECT value FROM json_each(?))"#,
            ids
        )
        .fetch_all(&self.0)
        .await
        .map_err(Arc::new)?;

        Ok(authors.into_iter().map(|author| (author.id, author)).collect())
    }
}

// All books of an author, by author id. Used by `Author.books`.
pub struct BooksByAuthorLoader(pub SqlitePool);

impl Loader<i64> for BooksByAuthorLoader {
    type Value = Vec<Book>;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, author_ids: &[i64]) -> Result<HashMap<i64, Vec<Book>>, Self::Error> {
        let author_ids = json_ids(author_ids);
        let books = sqlx::query_as!(
            Book,
            "SELECT id, title, year, author_id, created_by FROM books
             WHERE author_id IN (SELECT value FROM json_each(?))
             ORDER BY id",
            author_ids
        )
        .fetch_all(&self.0)
        .await
        .map_err(Arc::new)?;

        let mut by_author: HashMap<i64, Vec<Book>> = HashMap::new();
        for book in books {
            by_author.entry(book.author_id).or_default().push(book);
        }
        Ok(by_author)
    }
}
