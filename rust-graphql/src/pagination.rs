use async_graphql::connection::{self, Connection, Edge, OpaqueCursor};
use async_graphql::{Error, ID, OutputType, Result, SimpleObject};
use sqlx::SqlitePool;

use crate::models::{Author, Book};

// Cursor pagination (Relay-style connections).
//
// Offset pagination says "skip 20 rows"; a cursor says "continue after this row".
// Rows added or deleted between two requests cannot shift a cursor page.
// The cursor here is the row id, encoded so that clients treat it as opaque.

pub const DEFAULT_PAGE_SIZE: i64 = 10;
pub const MAX_PAGE_SIZE: i64 = 50;

type Cursor = OpaqueCursor<i64>;

// Extra field on every connection, next to `edges` and `pageInfo`.
#[derive(SimpleObject)]
pub struct ConnectionTotals {
    pub total_count: i64,
}

pub type BookConnection = Connection<Cursor, Book, ConnectionTotals>;
pub type AuthorConnection = Connection<Cursor, Author, ConnectionTotals>;

// Optional filters for book lists. `None` means "do not filter on this".
#[derive(Default)]
pub struct BookFilter {
    pub author_id: Option<i64>,
    pub title_contains: Option<String>,
}

impl BookFilter {
    pub fn new(author_id: Option<ID>, title_contains: Option<String>) -> Result<Self> {
        Ok(Self {
            author_id: author_id.map(|id| id.parse::<i64>()).transpose()?,
            // A blank search text filters nothing.
            title_contains: title_contains
                .map(|text| text.trim().to_string())
                .filter(|text| !text.is_empty()),
        })
    }
}

// How many items a connection field may return, for the query complexity limit.
pub fn page_cost(first: Option<i32>, last: Option<i32>) -> usize {
    let size = first.or(last).map_or(DEFAULT_PAGE_SIZE, i64::from);
    size.clamp(1, MAX_PAGE_SIZE) as usize
}

// The slice of rows a client asked for.
struct Window {
    after: Option<i64>,
    before: Option<i64>,
    limit: i64,
    // `last` was given: take rows from the end of the range instead of the start.
    backward: bool,
}

impl Window {
    fn new(
        after: Option<Cursor>,
        before: Option<Cursor>,
        first: Option<usize>,
        last: Option<usize>,
    ) -> Result<Self> {
        let (limit, backward) = match (first, last) {
            (Some(_), Some(_)) => return Err(Error::new("Use either first or last, not both")),
            (None, Some(last)) => (last as i64, true),
            (Some(first), None) => (first as i64, false),
            (None, None) => (DEFAULT_PAGE_SIZE, false),
        };
        if !(1..=MAX_PAGE_SIZE).contains(&limit) {
            return Err(Error::new("Page size should be between 1 and 50"));
        }
        Ok(Self {
            after: after.map(|cursor| cursor.0),
            before: before.map(|cursor| cursor.0),
            limit,
            backward,
        })
    }

    // One row more than requested, to learn whether another page exists.
    fn fetch_limit(&self) -> i64 {
        self.limit + 1
    }

    // `rows` holds up to `fetch_limit` rows: ascending by id when going
    // forward, descending when going backward.
    fn into_connection<T: OutputType>(
        self,
        mut rows: Vec<T>,
        total_count: i64,
        id: impl Fn(&T) -> i64,
    ) -> Connection<Cursor, T, ConnectionTotals> {
        let has_more = rows.len() as i64 > self.limit;
        rows.truncate(self.limit as usize);

        // A cursor on the other side means the client came from there, so
        // rows exist in that direction. This avoids a second query.
        let (has_previous_page, has_next_page) = if self.backward {
            rows.reverse();
            (has_more, self.before.is_some())
        } else {
            (self.after.is_some(), has_more)
        };

        let mut connection = Connection::with_additional_fields(
            has_previous_page,
            has_next_page,
            ConnectionTotals { total_count },
        );
        connection
            .edges
            .extend(rows.into_iter().map(|row| Edge::new(OpaqueCursor(id(&row)), row)));
        connection
    }
}

pub async fn count_books(pool: &SqlitePool, filter: &BookFilter) -> Result<i64> {
    let count = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM books
           WHERE (?1 IS NULL OR author_id = ?1)
             AND (?2 IS NULL OR instr(lower(title), lower(?2)) > 0)"#,
        filter.author_id,
        filter.title_contains
    )
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn book_connection(
    pool: &SqlitePool,
    filter: BookFilter,
    after: Option<String>,
    before: Option<String>,
    first: Option<i32>,
    last: Option<i32>,
) -> Result<BookConnection> {
    // connection::query decodes the cursors and rejects negative sizes.
    connection::query(after, before, first, last, |after, before, first, last| async move {
        let window = Window::new(after, before, first, last)?;
        let limit = window.fetch_limit();

        let rows = if window.backward {
            sqlx::query_as!(
                Book,
                "SELECT id, title, year, author_id, created_by FROM books
                 WHERE (?1 IS NULL OR author_id = ?1)
                   AND (?2 IS NULL OR instr(lower(title), lower(?2)) > 0)
                   AND (?3 IS NULL OR id > ?3)
                   AND (?4 IS NULL OR id < ?4)
                 ORDER BY id DESC
                 LIMIT ?5",
                filter.author_id,
                filter.title_contains,
                window.after,
                window.before,
                limit
            )
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as!(
                Book,
                "SELECT id, title, year, author_id, created_by FROM books
                 WHERE (?1 IS NULL OR author_id = ?1)
                   AND (?2 IS NULL OR instr(lower(title), lower(?2)) > 0)
                   AND (?3 IS NULL OR id > ?3)
                   AND (?4 IS NULL OR id < ?4)
                 ORDER BY id
                 LIMIT ?5",
                filter.author_id,
                filter.title_contains,
                window.after,
                window.before,
                limit
            )
            .fetch_all(pool)
            .await?
        };

        let total_count = count_books(pool, &filter).await?;
        Ok::<_, Error>(window.into_connection(rows, total_count, |book| book.id))
    })
    .await
}

pub async fn author_connection(
    pool: &SqlitePool,
    after: Option<String>,
    before: Option<String>,
    first: Option<i32>,
    last: Option<i32>,
) -> Result<AuthorConnection> {
    connection::query(after, before, first, last, |after, before, first, last| async move {
        let window = Window::new(after, before, first, last)?;
        let limit = window.fetch_limit();

        let rows = if window.backward {
            sqlx::query_as!(
                Author,
                "SELECT id, name FROM authors
                 WHERE (?1 IS NULL OR id > ?1)
                   AND (?2 IS NULL OR id < ?2)
                 ORDER BY id DESC
                 LIMIT ?3",
                window.after,
                window.before,
                limit
            )
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as!(
                Author,
                "SELECT id, name FROM authors
                 WHERE (?1 IS NULL OR id > ?1)
                   AND (?2 IS NULL OR id < ?2)
                 ORDER BY id
                 LIMIT ?3",
                window.after,
                window.before,
                limit
            )
            .fetch_all(pool)
            .await?
        };

        let total_count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM authors"#)
            .fetch_one(pool)
            .await?;
        Ok::<_, Error>(window.into_connection(rows, total_count, |author| author.id))
    })
    .await
}
