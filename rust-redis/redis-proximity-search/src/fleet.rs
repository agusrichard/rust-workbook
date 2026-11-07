use std::time::{Duration, SystemTime, UNIX_EPOCH};

use redis::{Cmd, Commands, Connection, RedisResult, Script};

// Where every driver is. A Geo key is a Sorted Set: member = driver, score = 52-bit geohash
const GEO_KEY: &str = "drivers:geo";
// When every driver last reported in. Sorted Set: member = driver, score = time in ms.
// Geo members can't have their own TTL, so this is how we spot drivers whose app went quiet
const SEEN_KEY: &str = "drivers:seen";
// Who is driving whom. Hash: field = driver, value = rider
const RIDES_KEY: &str = "drivers:rides";

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is before 1970")
        .as_millis() as u64
}

/// A driver found by a search, and how far they are from where the search started.
#[derive(Debug, Clone)]
pub struct Hit {
    pub driver: String,
    pub km: f64,
}

/// Drivers inside the search area, nearest first, sorted by whether a rider can book them.
#[derive(Debug, Default)]
pub struct Search {
    pub available: Vec<Hit>,
    /// On a ride right now.
    pub busy: Vec<Hit>,
    /// Still on the map, but haven't reported in recently.
    pub offline: Vec<Hit>,
}

pub struct Fleet {
    redis: Connection,
    /// A driver who hasn't reported in for this long counts as offline.
    stale_after: Duration,
    report: Script,
    sweep: Script,
}

impl Fleet {
    pub fn new(redis: Connection, stale_after: Duration) -> Self {
        Self {
            redis,
            stale_after,
            report: Script::new(include_str!("report.lua")),
            sweep: Script::new(include_str!("sweep.lua")),
        }
    }

    /// Wipes every driver so each demo run starts from scratch.
    pub fn reset(&mut self) -> RedisResult<()> {
        self.redis.del(&[GEO_KEY, SEEN_KEY, RIDES_KEY])
    }

    /// The driver's app reports where they are. Adds them to the map, or moves them if they're
    /// already on it, and counts as a heartbeat. Returns true if they were new to the map.
    /// Note the order: longitude first, then latitude.
    pub fn report(&mut self, driver: &str, lon: f64, lat: f64) -> RedisResult<bool> {
        let added: u8 = self
            .report
            .key(GEO_KEY)
            .key(SEEN_KEY)
            .arg(lon)
            .arg(lat)
            .arg(driver)
            // The app server's clock. With many servers, keep their clocks in sync or use Redis TIME
            .arg(now_millis())
            .invoke(&mut self.redis)?;
        Ok(added == 1)
    }

    /// The driver logs off. Returns false if they weren't on the map.
    pub fn leave(&mut self, driver: &str) -> RedisResult<bool> {
        let (removed,): (bool,) = redis::pipe()
            .atomic()
            // There is no GEODEL: a Geo key is a Sorted Set, so ZREM removes a member
            .zrem(GEO_KEY, driver)
            .zrem(SEEN_KEY, driver)
            .ignore()
            .hdel(RIDES_KEY, driver)
            .ignore()
            .query(&mut self.redis)?;
        Ok(removed)
    }

    /// How many drivers are on the map.
    pub fn drivers(&mut self) -> RedisResult<usize> {
        self.redis.zcard(GEO_KEY)
    }

    /// How many drivers have a heartbeat on record.
    pub fn heartbeats(&mut self) -> RedisResult<usize> {
        self.redis.zcard(SEEN_KEY)
    }

    /// The coordinates Redis stored for a driver, as (longitude, latitude).
    /// They come back slightly off from what was sent, because Redis keeps only the geohash.
    pub fn position(&mut self, driver: &str) -> RedisResult<Option<(f64, f64)>> {
        // GEOPOS takes many members and replies with one entry each, nil for unknown ones
        let mut positions: Vec<Option<(f64, f64)>> = redis::cmd("GEOPOS")
            .arg(GEO_KEY)
            .arg(driver)
            .query(&mut self.redis)?;
        Ok(positions.pop().flatten())
    }

    /// Distance between two drivers in km. None if either isn't on the map.
    pub fn distance(&mut self, a: &str, b: &str) -> RedisResult<Option<f64>> {
        redis::cmd("GEODIST")
            .arg(GEO_KEY)
            .arg(a)
            .arg(b)
            .arg("km")
            .query(&mut self.redis)
    }

    /// The driver's position as a standard 11-character geohash string.
    pub fn geohash(&mut self, driver: &str) -> RedisResult<Option<String>> {
        let mut hashes: Vec<Option<String>> = redis::cmd("GEOHASH")
            .arg(GEO_KEY)
            .arg(driver)
            .query(&mut self.redis)?;
        Ok(hashes.pop().flatten())
    }

    /// The raw Sorted Set score behind the driver's position: a 52-bit integer.
    pub fn score(&mut self, driver: &str) -> RedisResult<Option<u64>> {
        // 52 bits fit exactly in an f64, so this cast loses nothing
        let score: Option<f64> = self.redis.zscore(GEO_KEY, driver)?;
        Ok(score.map(|s| s as u64))
    }

    /// Every driver on the map within `radius_km` of a point, nearest first.
    /// Plain GEOSEARCH: it knows nothing about heartbeats or rides.
    /// `count` keeps only the nearest n.
    pub fn within(
        &mut self,
        lon: f64,
        lat: f64,
        radius_km: f64,
        count: Option<usize>,
    ) -> RedisResult<Vec<Hit>> {
        let mut cmd = redis::cmd("GEOSEARCH");
        cmd.arg(GEO_KEY)
            .arg("FROMLONLAT")
            .arg(lon)
            .arg(lat)
            .arg("BYRADIUS")
            .arg(radius_km)
            .arg("km");
        self.geosearch(cmd, count)
    }

    /// Other drivers within `radius_km` of `driver`, nearest first.
    /// Redis replies with an error if `driver` isn't on the map.
    pub fn around(&mut self, driver: &str, radius_km: f64) -> RedisResult<Vec<Hit>> {
        let mut cmd = redis::cmd("GEOSEARCH");
        cmd.arg(GEO_KEY)
            .arg("FROMMEMBER")
            .arg(driver)
            .arg("BYRADIUS")
            .arg(radius_km)
            .arg("km");
        let mut hits = self.geosearch(cmd, None)?;
        // The driver is 0 km from themselves, so they always come back first
        hits.retain(|hit| hit.driver != driver);
        Ok(hits)
    }

    /// Drivers inside a `width_km` × `height_km` rectangle centred on a point, such as
    /// the map on the rider's screen. Nearest to the centre first.
    pub fn in_box(
        &mut self,
        lon: f64,
        lat: f64,
        width_km: f64,
        height_km: f64,
    ) -> RedisResult<Vec<Hit>> {
        let mut cmd = redis::cmd("GEOSEARCH");
        cmd.arg(GEO_KEY)
            .arg("FROMLONLAT")
            .arg(lon)
            .arg(lat)
            .arg("BYBOX")
            .arg(width_km)
            .arg(height_km)
            .arg("km");
        self.geosearch(cmd, None)
    }

    fn geosearch(&mut self, mut cmd: Cmd, count: Option<usize>) -> RedisResult<Vec<Hit>> {
        // Without ASC the order follows the geohash cells, not distance
        cmd.arg("ASC");
        if let Some(n) = count {
            // Redis rejects COUNT 0 with an error. Nobody asked for anyone, so return nobody
            if n == 0 {
                return Ok(Vec::new());
            }
            cmd.arg("COUNT").arg(n);
        }
        // Each row becomes [driver, distance] instead of just the driver
        cmd.arg("WITHDIST");
        let rows: Vec<(String, f64)> = cmd.query(&mut self.redis)?;
        Ok(rows
            .into_iter()
            .map(|(driver, km)| Hit { driver, km })
            .collect())
    }

    /// Drivers within `radius_km` of a point, split into available, busy and offline.
    pub fn search(&mut self, lon: f64, lat: f64, radius_km: f64) -> RedisResult<Search> {
        // No COUNT here. COUNT runs inside Redis before our filter below, so "COUNT 5"
        // could return 5 busy drivers and leave the rider with nobody to book
        let hits = self.within(lon, lat, radius_km, None)?;
        if hits.is_empty() {
            return Ok(Search::default());
        }
        let drivers: Vec<&str> = hits.iter().map(|hit| hit.driver.as_str()).collect();

        // One round trip, two commands, each answering for every driver at once
        let (seen, rides): (Vec<Option<f64>>, Vec<Option<String>>) = redis::pipe()
            .cmd("ZMSCORE")
            .arg(SEEN_KEY)
            .arg(&drivers)
            .cmd("HMGET")
            .arg(RIDES_KEY)
            .arg(&drivers)
            .query(&mut self.redis)?;

        let cutoff = now_millis().saturating_sub(self.stale_after.as_millis() as u64) as f64;
        let mut search = Search::default();
        for ((hit, seen), ride) in hits.into_iter().zip(seen).zip(rides) {
            // No heartbeat on record counts as offline too
            if seen.is_none_or(|ms| ms < cutoff) {
                search.offline.push(hit);
            } else if ride.is_some() {
                search.busy.push(hit);
            } else {
                search.available.push(hit);
            }
        }
        Ok(search)
    }

    /// Books the nearest available driver within `radius_km` for `rider`.
    /// None if nobody nearby is free.
    pub fn book(&mut self, rider: &str, lon: f64, lat: f64, radius_km: f64) -> RedisResult<Option<Hit>> {
        for hit in self.search(lon, lat, radius_km)?.available {
            // Another rider may have booked this driver since our search. HSETNX writes
            // only if the field is missing, so exactly one rider wins. Losers try the next driver
            let booked: bool = self.redis.hset_nx(RIDES_KEY, &hit.driver, rider)?;
            if booked {
                return Ok(Some(hit));
            }
        }
        Ok(None)
    }

    /// The rider a driver is driving, if any.
    pub fn rider_of(&mut self, driver: &str) -> RedisResult<Option<String>> {
        self.redis.hget(RIDES_KEY, driver)
    }

    /// Ends the driver's ride, making them available again. Returns false if they had no ride.
    pub fn finish(&mut self, driver: &str) -> RedisResult<bool> {
        self.redis.hdel(RIDES_KEY, driver)
    }

    /// Takes drivers who stopped reporting off the map. Returns who was removed.
    /// Searches already skip them, so this only frees memory. Run it now and then.
    pub fn sweep(&mut self) -> RedisResult<Vec<String>> {
        let cutoff = now_millis().saturating_sub(self.stale_after.as_millis() as u64);
        self.sweep
            .key(GEO_KEY)
            .key(SEEN_KEY)
            .arg(cutoff)
            .invoke(&mut self.redis)
    }
}
