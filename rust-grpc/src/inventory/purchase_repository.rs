use sqlx::SqlitePool;
use std::error::Error;
use std::fmt::{Display, Formatter};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Debug)]
pub struct Purchase {
    pub id: i64,
    pub product_id: i64,
    pub purchased_quantity: i64,
    pub remaining_stock: i64,
    pub purchased_at: String,
}

#[derive(Debug)]
pub enum PurchaseError {
    InvalidQuantity,
    ProductNotFound,
    InsufficientStock,
    Database(sqlx::Error),
    Timestamp(time::error::Format),
}

impl Display for PurchaseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidQuantity => write!(formatter, "quantity must be greater than zero"),
            Self::ProductNotFound => write!(formatter, "product not found"),
            Self::InsufficientStock => write!(formatter, "insufficient stock"),
            Self::Database(error) => write!(formatter, "database error: {error}"),
            Self::Timestamp(error) => write!(formatter, "timestamp formatting error: {error}"),
        }
    }
}

impl Error for PurchaseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::Timestamp(error) => Some(error),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for PurchaseError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<time::error::Format> for PurchaseError {
    fn from(error: time::error::Format) -> Self {
        Self::Timestamp(error)
    }
}

#[derive(Clone)]
pub struct PurchaseRepository {
    pool: SqlitePool,
}

impl PurchaseRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, product_id: i64, quantity: i64) -> Result<Purchase, PurchaseError> {
        if quantity <= 0 {
            return Err(PurchaseError::InvalidQuantity);
        }

        let purchased_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
        let mut transaction = self.pool.begin().await?;

        let update_result =
            sqlx::query("UPDATE products SET stock = stock - ? WHERE id = ? AND stock >= ?")
                .bind(quantity)
                .bind(product_id)
                .bind(quantity)
                .execute(&mut *transaction)
                .await?;

        if update_result.rows_affected() == 0 {
            let stock = sqlx::query_scalar::<_, i64>("SELECT stock FROM products WHERE id = ?")
                .bind(product_id)
                .fetch_optional(&mut *transaction)
                .await?;

            let error = match stock {
                None => PurchaseError::ProductNotFound,
                Some(_) => PurchaseError::InsufficientStock,
            };

            transaction.rollback().await?;
            return Err(error);
        }

        let remaining_stock =
            sqlx::query_scalar::<_, i64>("SELECT stock FROM products WHERE id = ?")
                .bind(product_id)
                .fetch_one(&mut *transaction)
                .await?;

        let insert_result = sqlx::query(
            "INSERT INTO purchases (product_id, quantity, purchased_at) VALUES (?, ?, ?)",
        )
        .bind(product_id)
        .bind(quantity)
        .bind(&purchased_at)
        .execute(&mut *transaction)
        .await?;

        let purchase_id = insert_result.last_insert_rowid();
        transaction.commit().await?;

        Ok(Purchase {
            id: purchase_id,
            product_id,
            purchased_quantity: quantity,
            remaining_stock,
            purchased_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn product_stock(pool: &SqlitePool, product_id: i64) -> i64 {
        sqlx::query_scalar("SELECT stock FROM products WHERE id = ?")
            .bind(product_id)
            .fetch_one(pool)
            .await
            .expect("product stock should be readable")
    }

    async fn purchase_count(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM purchases")
            .fetch_one(pool)
            .await
            .expect("purchase count should be readable")
    }

    #[sqlx::test]
    async fn create_purchase_decreases_stock_and_inserts_purchase(pool: SqlitePool) {
        let repository = PurchaseRepository::new(pool.clone());

        let purchase = repository
            .create(1, 3)
            .await
            .expect("purchase should succeed");

        assert!(purchase.id > 0);
        assert_eq!(purchase.product_id, 1);
        assert_eq!(purchase.purchased_quantity, 3);
        assert_eq!(purchase.remaining_stock, 7);
        assert_eq!(product_stock(&pool, 1).await, 7);
        assert_eq!(purchase_count(&pool).await, 1);
        OffsetDateTime::parse(&purchase.purchased_at, &Rfc3339)
            .expect("purchase timestamp should be RFC 3339");

        let stored_purchase = sqlx::query_as::<_, (i64, i64, i64, String)>(
            "SELECT id, product_id, quantity, purchased_at FROM purchases WHERE id = ?",
        )
        .bind(purchase.id)
        .fetch_one(&pool)
        .await
        .expect("purchase should be stored");

        assert_eq!(stored_purchase.0, purchase.id);
        assert_eq!(stored_purchase.1, purchase.product_id);
        assert_eq!(stored_purchase.2, purchase.purchased_quantity);
        assert_eq!(stored_purchase.3, purchase.purchased_at);
    }

    #[sqlx::test]
    async fn create_purchase_rejects_unknown_product_without_changes(pool: SqlitePool) {
        let repository = PurchaseRepository::new(pool.clone());

        let result = repository.create(999, 1).await;

        assert!(matches!(result, Err(PurchaseError::ProductNotFound)));
        assert_eq!(product_stock(&pool, 1).await, 10);
        assert_eq!(purchase_count(&pool).await, 0);
    }

    #[sqlx::test]
    async fn create_purchase_rejects_zero_quantity_without_changes(pool: SqlitePool) {
        let repository = PurchaseRepository::new(pool.clone());

        let result = repository.create(1, 0).await;

        assert!(matches!(result, Err(PurchaseError::InvalidQuantity)));
        assert_eq!(product_stock(&pool, 1).await, 10);
        assert_eq!(purchase_count(&pool).await, 0);
    }

    #[sqlx::test]
    async fn create_purchase_rejects_insufficient_stock_without_changes(pool: SqlitePool) {
        let repository = PurchaseRepository::new(pool.clone());

        let result = repository.create(1, 11).await;

        assert!(matches!(result, Err(PurchaseError::InsufficientStock)));
        assert_eq!(product_stock(&pool, 1).await, 10);
        assert_eq!(purchase_count(&pool).await, 0);
    }

    #[sqlx::test]
    async fn concurrent_purchases_do_not_oversell_product(pool: SqlitePool) {
        let repository = PurchaseRepository::new(pool.clone());

        let (first, second) = tokio::join!(repository.create(1, 6), repository.create(1, 6));
        let results = [first, second];

        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Err(PurchaseError::InsufficientStock)))
                .count(),
            1
        );
        assert_eq!(product_stock(&pool, 1).await, 4);
        assert_eq!(purchase_count(&pool).await, 1);
        assert_eq!(10, 6 + product_stock(&pool, 1).await);
    }
}
