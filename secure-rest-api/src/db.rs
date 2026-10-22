use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;

pub async fn connect(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;

    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}
