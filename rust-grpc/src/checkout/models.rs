use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ProductResponse {
    pub product_id: i64,
    pub name: String,
    pub stock: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreatePurchaseRequest {
    pub product_id: i64,
    pub quantity: i64,
}

#[derive(Debug, Serialize)]
pub struct CreatePurchaseResponse {
    pub purchase_id: i64,
    pub product_id: i64,
    pub purchased_quantity: i64,
    pub remaining_stock: i64,
    pub purchased_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PurchaseNotification {
    pub purchase_id: i64,
    pub product_id: i64,
    pub quantity: i64,
    pub purchased_at: String,
}
