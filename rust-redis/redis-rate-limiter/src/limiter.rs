use std::time::{Duration, SystemTime, UNIX_EPOCH};

use redis::{Connection, RedisResult, Script};

/// What the limiter decided about one request.
pub struct Decision {
    pub allowed: bool,
    /// Requests still allowed right now.
    pub remaining: u64,
    /// How long until a request would be allowed again. Zero when allowed.
    pub retry_after: Duration,
}

/// Anything that can decide whether a client's request goes through.
pub trait RateLimiter {
    fn hit(&mut self, client: &str) -> RedisResult<Decision>;
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is before 1970")
        .as_millis() as u64
}

pub struct FixedWindow {
    redis: Connection,
    limit: u64,
    window: Duration,
}

impl FixedWindow {
    pub fn new(redis: Connection, limit: u64, window: Duration) -> Self {
        Self {
            redis,
            limit,
            window,
        }
    }

    fn key(client: &str, window_start_ms: u64) -> String {
        format!("rate:fixed:{client}:{window_start_ms}")
    }
}

impl RateLimiter for FixedWindow {
    fn hit(&mut self, client: &str) -> RedisResult<Decision> {
        let now = now_millis();
        let window_ms = self.window.as_millis() as u64;
        let start = now - now % window_ms;
        let key = Self::key(client, start);

        // MULTI/EXEC, so a counter can never be left without an expiry
        let (count,): (u64,) = redis::pipe()
            .atomic()
            .cmd("INCR")
            .arg(&key)
            // NX: the first request sets the expiry, later ones leave it alone
            .cmd("PEXPIRE")
            .arg(&key)
            .arg(window_ms)
            .arg("NX")
            .ignore()
            .query(&mut self.redis)?;

        let allowed = count <= self.limit;
        Ok(Decision {
            allowed,
            remaining: self.limit.saturating_sub(count),
            retry_after: if allowed {
                Duration::ZERO
            } else {
                Duration::from_millis(start + window_ms - now)
            },
        })
    }
}

pub struct SlidingWindowLog {
    redis: Connection,
    limit: u64,
    window: Duration,
}

impl SlidingWindowLog {
    pub fn new(redis: Connection, limit: u64, window: Duration) -> Self {
        Self {
            redis,
            limit,
            window,
        }
    }

    fn key(client: &str) -> String {
        format!("rate:sliding:{client}")
    }
}

impl RateLimiter for SlidingWindowLog {
    fn hit(&mut self, client: &str) -> RedisResult<Decision> {
        let now = now_millis();
        let window_ms = self.window.as_millis() as u64;
        let key = Self::key(client);

        // Round trip 1, read: forget requests that slid out of the window, then look at the rest
        let (count, oldest): (u64, Vec<(String, u64)>) = redis::pipe()
            .atomic()
            // Scores are timestamps, so this drops everything at or before `now - window`
            .cmd("ZREMRANGEBYSCORE")
            .arg(&key)
            .arg("-inf")
            .arg(now - window_ms)
            .ignore()
            .cmd("ZCARD")
            .arg(&key)
            // Lowest score = oldest request still in the window
            .cmd("ZRANGE")
            .arg(&key)
            .arg(0)
            .arg(0)
            .arg("WITHSCORES")
            .query(&mut self.redis)?;

        if count >= self.limit {
            // Blocked requests are not logged, so retrying doesn't push the unblock time back.
            // A slot frees up when the oldest request slides out of the window.
            let oldest_ms = oldest.first().map_or(now, |(_, ts)| *ts);
            return Ok(Decision {
                allowed: false,
                remaining: 0,
                retry_after: Duration::from_millis((oldest_ms + window_ms).saturating_sub(now)),
            });
        }

        // Round trip 2, write: log this request.
        // GAP: another request for the same client can run round trip 1 between our two
        // round trips, see the same count, and also get in. `SlidingWindowLua` closes it.
        // Members must be unique, and a burst can land several requests in the same millisecond
        let member = format!("{now}-{:x}", rand::random::<u64>());
        redis::pipe()
            .atomic()
            .cmd("ZADD")
            .arg(&key)
            .arg(now)
            .arg(member)
            .ignore()
            // No NX this time: the log must outlive its newest entry, so every request pushes it back
            .cmd("PEXPIRE")
            .arg(&key)
            .arg(window_ms)
            .ignore()
            .exec(&mut self.redis)?;

        Ok(Decision {
            allowed: true,
            remaining: self.limit - count - 1,
            retry_after: Duration::ZERO,
        })
    }
}

/// The same sliding window log, but the whole check-then-write runs as one Lua script,
/// so concurrent requests can't both see a free slot.
pub struct SlidingWindowLua {
    redis: Connection,
    limit: u64,
    window: Duration,
    script: Script,
}

impl SlidingWindowLua {
    pub fn new(redis: Connection, limit: u64, window: Duration) -> Self {
        Self {
            redis,
            limit,
            window,
            // Baked into the binary at compile time
            script: Script::new(include_str!("sliding_window.lua")),
        }
    }

    fn key(client: &str) -> String {
        format!("rate:lua:{client}")
    }
}

impl RateLimiter for SlidingWindowLua {
    fn hit(&mut self, client: &str) -> RedisResult<Decision> {
        let now = now_millis();
        let member = format!("{now}-{:x}", rand::random::<u64>());

        // One round trip. Sends EVALSHA (just the script's hash), and if Redis doesn't
        // have the script cached yet, loads it and retries
        let (allowed, remaining, retry_after_ms): (u8, u64, u64) = self
            .script
            .key(Self::key(client))
            .arg(now)
            .arg(self.window.as_millis() as u64)
            .arg(self.limit)
            .arg(member)
            .invoke(&mut self.redis)?;

        Ok(Decision {
            allowed: allowed == 1,
            remaining,
            retry_after: Duration::from_millis(retry_after_ms),
        })
    }
}
