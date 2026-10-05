use std::str::FromStr;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

pub async fn connect(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;

    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}