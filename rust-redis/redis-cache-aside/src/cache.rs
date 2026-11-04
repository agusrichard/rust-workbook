use std::format;

use redis::{Commands, Connection, ErrorKind, RedisError, RedisResult};

use crate::db::{FakeDb, Product};

const TTL_SECS: u64 = 60;

fn key(id: u32) -> String {
    format!("product:{id}")
}

pub struct ProductService {
    db: FakeDb,
    redis: Connection,
}

impl ProductService {
    pub fn new(db: FakeDb, redis: Connection) -> Self {
        Self { db, redis }
    }

    pub fn get(&mut self, id: u32) -> RedisResult<Option<Product>> {
        let cached: Option<String> = self.redis.get(key(id))?;
        if let Some(json) = cached
            && let Ok(product) = serde_json::from_str(&json)
        {
            println!("  cache HIT");
            return Ok(Some(product));
        }

        println!("  cache MISS, loading from database");

        let product = self.db.find(id);
        if let Some(p) = &product {
            let json = serde_json::to_string(p)
                .map_err(|_| RedisError::from((ErrorKind::Client, "Error when serializing")))?;
            let _: () = self.redis.set_ex(key(id), json, TTL_SECS)?;
        }

        Ok(product)
    }

    pub fn update_price(&mut self, id: u32, price: u32) -> RedisResult<()> {
        self.db.update_price(id, price);
        let _: () = self.redis.del(key(id))?;
        Ok(())
    }
}
