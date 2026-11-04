use std::{collections::HashMap, thread::sleep, time::Duration};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: u32,
    pub name: String,
    pub price: u32,
}

pub struct FakeDb {
    products: HashMap<u32, Product>,
}

impl FakeDb {
    pub fn new() -> Self {
        let products = [
            (1, "Keyboard", 450),
            (2, "Mouse", 150),
            (3, "Monitor", 2_100),
        ]
        .into_iter()
        .map(|(id, name, price)| {
            (
                id,
                Product {
                    id,
                    name: name.to_string(),
                    price,
                },
            )
        })
        .collect();
        Self { products }
    }

    pub fn find(&self, id: u32) -> Option<Product> {
        sleep(Duration::from_millis(500));
        self.products.get(&id).cloned()
    }

    pub fn update_price(&mut self, id: u32, price: u32) {
        sleep(Duration::from_millis(500));
        if let Some(product) = self.products.get_mut(&id) {
            product.price = price;
        }
    }
}
