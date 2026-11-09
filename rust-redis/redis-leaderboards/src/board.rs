use redis::{Commands, Connection, RedisResult};

// A week of history plus today, so the weekly board can still be rebuilt from every day in it
const DAY_TTL_SECS: i64 = 8 * 24 * 60 * 60;
const WEEK_TTL_SECS: i64 = 5 * 7 * 24 * 60 * 60;

fn date_key(date: &str) -> String {
    format!("lb:{date}")
}

fn week_key(week: &str) -> String {
    format!("lb:week:{week}")
}

/// How a player's daily scores combine into their weekly score.
pub enum Aggregate {
    /// Their best single run of the week.
    Max,
    /// All their daily bests added up, so showing up every day pays off.
    Sum,
}

pub struct Board {
    redis: Connection,
}

impl Board {
    pub fn new(redis: Connection) -> Self {
        Self { redis }
    }

    /// Records a run. Returns true if it was the player's first score of the day or a new best.
    pub fn submit(&mut self, date: &str, player: &str, score: u64) -> RedisResult<bool> {
        let key = date_key(date);
        // MULTI/EXEC, so a board can never be left without an expiry
        let (best,): (bool,) = redis::pipe()
            .atomic()
            // GT keeps the higher score, CH makes the reply count improved scores, not just new players
            .cmd("ZADD")
            .arg(&key)
            .arg("GT")
            .arg("CH")
            .arg(score)
            .arg(player)
            // NX: the first score of the day starts the clock, later ones don't push it back
            .cmd("EXPIRE")
            .arg(&key)
            .arg(DAY_TTL_SECS)
            .arg("NX")
            .ignore()
            .query(&mut self.redis)?;
        Ok(best)
    }

    /// Wipes the day's board so every demo run starts from scratch.
    pub fn reset(&mut self, date: &str) -> RedisResult<()> {
        self.redis.del(date_key(date))
    }

    /// The `n` highest scores of the day, best first.
    pub fn top(&mut self, date: &str, n: isize) -> RedisResult<Vec<(String, u64)>> {
        self.top_of(&date_key(date), n)
    }

    /// The `n` highest scores of the week, best first. Build the week first with `build_weekly`.
    pub fn top_weekly(&mut self, week: &str, n: isize) -> RedisResult<Vec<(String, u64)>> {
        self.top_of(&week_key(week), n)
    }

    fn top_of(&mut self, key: &str, n: isize) -> RedisResult<Vec<(String, u64)>> {
        // The stop index would be -1, which Redis reads as "the whole board"
        if n <= 0 {
            return Ok(Vec::new());
        }
        // Rank ranges are inclusive, so 0..=n-1 is exactly n players
        self.redis.zrevrange_withscores(key, 0, n - 1)
    }

    /// How many players scored that day.
    pub fn players(&mut self, date: &str) -> RedisResult<usize> {
        self.redis.zcard(date_key(date))
    }

    /// The player's position, 0 = best. None if they haven't played that day.
    pub fn rank(&mut self, date: &str, player: &str) -> RedisResult<Option<usize>> {
        self.redis.zrevrank(date_key(date), player)
    }

    /// Players within `radius` places of `player`, best first, as (rank, player, score).
    /// Empty if the player hasn't played that day.
    pub fn around(
        &mut self,
        date: &str,
        player: &str,
        radius: usize,
    ) -> RedisResult<Vec<(usize, String, u64)>> {
        // Two round trips: a score landing in between can shift the window by a place.
        // Fine for a leaderboard; a Lua script would make it exact.
        let Some(rank) = self.rank(date, player)? else {
            return Ok(Vec::new());
        };
        // saturating_sub keeps the #1 player's window from going below rank 0.
        // Going past the last rank is fine, Redis just returns fewer players.
        let start = rank.saturating_sub(radius);
        let stop = rank + radius;
        let window: Vec<(String, u64)> =
            self.redis
                .zrevrange_withscores(date_key(date), start as isize, stop as isize)?;
        Ok(window
            .into_iter()
            .enumerate()
            .map(|(i, (player, score))| (start + i, player, score))
            .collect())
    }

    /// Seconds until the day's board expires, or a negative number if it doesn't exist.
    pub fn ttl(&mut self, date: &str) -> RedisResult<i64> {
        self.redis.ttl(date_key(date))
    }

    /// Merges the daily boards into the week's board, replacing it. Days nobody played count as empty.
    /// Returns how many players made the weekly board.
    pub fn build_weekly(
        &mut self,
        week: &str,
        dates: &[String],
        aggregate: Aggregate,
    ) -> RedisResult<usize> {
        let key = week_key(week);
        let days: Vec<String> = dates.iter().map(|date| date_key(date)).collect();
        let aggregate = match aggregate {
            Aggregate::Max => "MAX",
            Aggregate::Sum => "SUM",
        };
        let (players,): (usize,) = redis::pipe()
            .atomic()
            .cmd("ZUNIONSTORE")
            .arg(&key)
            .arg(days.len())
            .arg(&days)
            .arg("AGGREGATE")
            .arg(aggregate)
            .cmd("EXPIRE")
            .arg(&key)
            .arg(WEEK_TTL_SECS)
            .ignore()
            .query(&mut self.redis)?;
        Ok(players)
    }
}
