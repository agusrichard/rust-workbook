use sqlx::{FromRow, Result, SqlitePool};

#[derive(Debug, FromRow)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub stock: i64,
}

pub struct ProductRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ProductRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        ProductRepository { pool }
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<Product>, sqlx::Error> {
        let result = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;
        Ok(result)
    }

    pub async fn get_all(&self) -> Result<Vec<Product>, sqlx::Error> {
        let result = sqlx::query_as::<_, Product>("SELECT * FROM products ORDER BY id")
            .fetch_all(self.pool)
            .await?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn get_by_id_returns_existing_product(pool: SqlitePool) -> Result<()> {
        let repository = ProductRepository::new(&pool);
        let product = repository.get_by_id(1).await?;
        assert!(product.is_some());

        let product = product.unwrap();
        assert_eq!(product.id, 1);
        assert_eq!(product.name, "television");
        assert_eq!(product.stock, 10);

        Ok(())
    }

    #[sqlx::test]
    async fn get_by_id_returns_none_for_unkown_product(pool: SqlitePool) -> Result<()> {
        let repository = ProductRepository::new(&pool);
        let product = repository.get_by_id(99).await?;
        assert!(product.is_none());

        Ok(())
    }
}
