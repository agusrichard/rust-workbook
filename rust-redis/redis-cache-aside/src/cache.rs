use std::{eprintln, format, sync::Arc, thread::sleep, time::Duration};

use redis::{Commands, Connection, ErrorKind, RedisError, RedisResult, Script};

use crate::db::{FakeDb, Product};

const TTL_SECS: u64 = 60;
const NOT_FOUND: &str = "null";
const NOT_FOUND_TTL_SECS: u64 = 10;
const TTL_JITTER_RANGE: u64 = 5;

// Must be longer than a database load, or a second request would start loading too
const LOCK_TTL_MS: u64 = 2_000;
const LOCK_RETRIES: u32 = 20;
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(50);

const STATS_HITS: &str = "stats:hits";
const STATS_MISSES: &str = "stats:misses";

// Deletes the lock only if it still holds our token, so we never free someone else's lock
const UNLOCK_SCRIPT: &str = r#"
if redis.call("GET", KEYS[1]) == ARGV[1] then
  return redis.call("DEL", KEYS[1])
end
return 0
"#;

fn key(id: u32) -> String {
    format!("product:{id}")
}

fn lock_key(id: u32) -> String {
    format!("lock:product:{id}")
}

fn generate_ttl() -> u64 {
    rand::random_range(TTL_SECS - TTL_JITTER_RANGE..=TTL_SECS + TTL_JITTER_RANGE)
}

pub struct ProductService {
    db: Arc<FakeDb>,
    redis: Connection,
}

impl ProductService {
    pub fn new(db: Arc<FakeDb>, redis: Connection) -> Self {
        Self { db, redis }
    }

    pub fn get(&mut self, id: u32) -> RedisResult<Option<Product>> {
        if let Some(product) = self.read_cache(id) {
            self.hit("cache HIT");
            return Ok(product);
        }

        // On a miss, only the request holding the lock loads from the database
        let lock = lock_key(id);
        let token = rand::random::<u64>().to_string();
        if self.acquire_lock(&lock, &token) {
            // Another request may have filled the cache just before we got the lock
            let result = match self.read_cache(id) {
                Some(product) => {
                    self.hit("cache HIT");
                    Ok(product)
                }
                None => self.load_from_db(id),
            };
            self.release_lock(&lock, &token);
            return result;
        }

        println!("  waiting for another request to load it");
        for _ in 0..LOCK_RETRIES {
            sleep(LOCK_RETRY_DELAY);
            if let Some(product) = self.read_cache(id) {
                self.hit("cache HIT after waiting");
                return Ok(product);
            }
        }

        println!("  gave up waiting");
        self.load_from_db(id)
    }

    pub fn update_price(&mut self, id: u32, price: u32) -> RedisResult<()> {
        self.db.update_price(id, price);
        self.invalidate(id)
    }

    pub fn invalidate(&mut self, id: u32) -> RedisResult<()> {
        let _: () = self.redis.del(key(id))?;
        Ok(())
    }

    /// Returns (hits, misses), counted across every process using this cache.
    pub fn stats(&mut self) -> RedisResult<(u64, u64)> {
        let (hits, misses): (Option<u64>, Option<u64>) =
            self.redis.mget(&[STATS_HITS, STATS_MISSES])?;
        Ok((hits.unwrap_or(0), misses.unwrap_or(0)))
    }

    pub fn reset_stats(&mut self) -> RedisResult<()> {
        let _: () = self.redis.del(&[STATS_HITS, STATS_MISSES])?;
        Ok(())
    }

    /// Outer `None`: not in the cache. `Some(None)`: cached as "does not exist".
    fn read_cache(&mut self, id: u32) -> Option<Option<Product>> {
        let cached: Option<String> = match self.redis.get(key(id)) {
            Ok(value) => value,
            Err(e) => {
                eprintln!("     redis read failed, using database: {e}");
                None
            }
        };
        match cached.as_deref() {
            Some(NOT_FOUND) => Some(None),
            Some(json) => serde_json::from_str(json).ok().map(Some),
            None => None,
        }
    }

    fn load_from_db(&mut self, id: u32) -> RedisResult<Option<Product>> {
        println!("  cache MISS, loading from database");
        self.record(STATS_MISSES);
        let product = self.db.find(id);
        self.write_cache(id, product.as_ref())?;
        Ok(product)
    }

    fn write_cache(&mut self, id: u32, product: Option<&Product>) -> RedisResult<()> {
        let result = match product {
            Some(p) => {
                let json = serde_json::to_string(p)
                    .map_err(|_| RedisError::from((ErrorKind::Client, "Error when serializing")))?;
                self.redis.set_ex::<_, _, ()>(key(id), json, generate_ttl())
            }
            None => self.redis.set_ex(key(id), NOT_FOUND, NOT_FOUND_TTL_SECS),
        };
        if let Err(e) = result {
            eprintln!("     redis write failed: {e}");
        }
        Ok(())
    }

    /// Returns true if this request may load from the database. If Redis is
    /// unreachable there is nothing to coordinate with, so it loads anyway.
    fn acquire_lock(&mut self, lock: &str, token: &str) -> bool {
        // SET NX replies "OK" when we get the lock and nil when someone else holds it
        let reply: RedisResult<Option<String>> = redis::cmd("SET")
            .arg(lock)
            .arg(token)
            .arg("NX")
            .arg("PX")
            .arg(LOCK_TTL_MS)
            .query(&mut self.redis);
        match reply {
            Ok(reply) => reply.is_some(),
            Err(e) => {
                eprintln!("     redis lock failed, loading anyway: {e}");
                true
            }
        }
    }

    fn release_lock(&mut self, lock: &str, token: &str) {
        let released: RedisResult<i32> = Script::new(UNLOCK_SCRIPT)
            .key(lock)
            .arg(token)
            .invoke(&mut self.redis);
        if let Err(e) = released {
            eprintln!("     redis unlock failed, lock expires on its own: {e}");
        }
    }

    fn hit(&mut self, message: &str) {
        println!("  {message}");
        self.record(STATS_HITS);
    }

    fn record(&mut self, counter: &str) {
        if let Err(e) = self.redis.incr::<_, _, ()>(counter, 1) {
            eprintln!("     redis stats update failed: {e}");
        }
    }
}
