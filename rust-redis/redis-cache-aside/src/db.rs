use std::{
    collections::HashMap,
    sync::{
        RwLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread::sleep,
    time::Duration,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: u32,
    pub name: String,
    pub price: u32,
}

/// Pretends to be a slow database. Safe to share between threads with `Arc`.
pub struct FakeDb {
    products: RwLock<HashMap<u32, Product>>,
    find_calls: AtomicUsize,
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
        Self {
            products: RwLock::new(products),
            find_calls: AtomicUsize::new(0),
        }
    }

    pub fn find(&self, id: u32) -> Option<Product> {
        self.find_calls.fetch_add(1, Ordering::Relaxed);
        sleep(Duration::from_millis(500));
        self.products
            .read()
            .expect("products lock poisoned")
            .get(&id)
            .cloned()
    }

    pub fn update_price(&self, id: u32, price: u32) {
        sleep(Duration::from_millis(500));
        if let Some(product) = self
            .products
            .write()
            .expect("products lock poisoned")
            .get_mut(&id)
        {
            product.price = price;
        }
    }

    /// How many times `find` has hit the "database" so far.
    pub fn find_calls(&self) -> usize {
        self.find_calls.load(Ordering::Relaxed)
    }
}
