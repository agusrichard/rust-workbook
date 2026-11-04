use std::{sync::Arc, thread::sleep, time::Duration};

use redis::{Commands, Connection, RedisError, RedisResult, Script};

use crate::db::{FakeDb, Seat};

const EVENT: &str = "concert";

// Real ticket sites give you about 10 minutes to pay. Short here so the demo runs quickly.
pub const HOLD_TTL: Duration = Duration::from_millis(1_000);

// Deletes the hold only if it still holds our token, so we never free someone else's hold
const RELEASE_SCRIPT: &str = r#"
if redis.call("GET", KEYS[1]) == ARGV[1] then
  return redis.call("DEL", KEYS[1])
end
return 0
"#;

fn hold_key(seat: &str) -> String {
    format!("hold:{EVENT}:{seat}")
}

/// Tokens look like "alice:1234", so anyone reading the hold can tell who owns it.
fn fan_of(token: &str) -> String {
    token.split(':').next().unwrap_or(token).to_string()
}

/// A fan's temporary claim on a seat while they pay.
#[derive(Debug)]
pub struct Hold {
    pub seat: String,
    pub fan: String,
    /// Unique per hold, so the same fan in two browser tabs gets two different holds.
    token: String,
}

/// Outcome of clicking a seat.
pub enum HoldResult {
    /// The seat is ours until `HOLD_TTL` runs out: go pay for it.
    Held(Hold),
    /// Another fan is paying for it right now.
    HeldBy(String),
    /// Already bought.
    Sold,
}

#[derive(Debug)]
pub enum CheckoutError {
    /// The hold ran out before payment started, so the card was not charged.
    HoldExpired,
    /// The hold ran out during payment and someone else bought the seat, so the card was refunded.
    SeatSold,
    Redis(RedisError),
}

impl From<RedisError> for CheckoutError {
    fn from(e: RedisError) -> Self {
        Self::Redis(e)
    }
}

pub enum SeatStatus {
    Available,
    Held { fan: String },
    Sold { fan: String },
}

pub struct BoxOffice {
    db: Arc<FakeDb>,
    redis: Connection,
}

impl BoxOffice {
    pub fn new(db: Arc<FakeDb>, redis: Connection) -> Self {
        Self { db, redis }
    }

    /// Unlike the cache, this fails closed: if Redis is down we cannot hold
    /// seats, so the error goes back to the fan instead of selling blindly.
    pub fn hold(&mut self, seat: &str, fan: &str) -> RedisResult<HoldResult> {
        if let Some(Seat::Sold { .. }) = self.db.seat(seat) {
            return Ok(HoldResult::Sold);
        }

        let token = format!("{fan}:{}", rand::random::<u64>());
        // NX sets the hold only if nobody has one. GET returns the current holder
        // in the same command, so nil means the seat is now ours.
        let current: Option<String> = redis::cmd("SET")
            .arg(hold_key(seat))
            .arg(&token)
            .arg("NX")
            .arg("PX")
            .arg(HOLD_TTL.as_millis() as u64)
            .arg("GET")
            .query(&mut self.redis)?;
        Ok(match current {
            None => HoldResult::Held(Hold {
                seat: seat.to_string(),
                fan: fan.to_string(),
                token,
            }),
            Some(other) => HoldResult::HeldBy(fan_of(&other)),
        })
    }

    pub fn checkout(&mut self, hold: &Hold, payment_time: Duration) -> Result<(), CheckoutError> {
        // Do not charge a card for a seat the fan no longer holds
        if !self.still_held(hold)? {
            return Err(CheckoutError::HoldExpired);
        }

        println!("  charging {}'s card...", hold.fan);
        sleep(payment_time);

        // The hold may have run out while the card was charging, so the database has the final say
        let booked = self.db.book(&hold.seat, &hold.fan);
        self.release(hold);
        if booked {
            Ok(())
        } else {
            Err(CheckoutError::SeatSold)
        }
    }

    /// Gives the seat back early. Returns false if the hold had already run out.
    pub fn cancel(&mut self, hold: &Hold) -> bool {
        self.release(hold)
    }

    pub fn seat_map(&mut self) -> RedisResult<Vec<(String, SeatStatus)>> {
        let seats = self.db.seats();
        let keys: Vec<String> = seats.iter().map(|(seat, _)| hold_key(seat)).collect();
        let holds: Vec<Option<String>> = self.redis.mget(&keys)?;

        Ok(seats
            .into_iter()
            .zip(holds)
            .map(|((seat, status), hold)| {
                let status = match (status, hold) {
                    (Seat::Sold { fan }, _) => SeatStatus::Sold { fan },
                    (Seat::Available, Some(token)) => SeatStatus::Held {
                        fan: fan_of(&token),
                    },
                    (Seat::Available, None) => SeatStatus::Available,
                };
                (seat, status)
            })
            .collect())
    }

    /// Removes every hold, for example ones left behind by an earlier run.
    pub fn clear_holds(&mut self) -> RedisResult<()> {
        let keys: Vec<String> = self
            .db
            .seats()
            .iter()
            .map(|(seat, _)| hold_key(seat))
            .collect();
        let _: () = self.redis.del(keys)?;
        Ok(())
    }

    fn still_held(&mut self, hold: &Hold) -> RedisResult<bool> {
        let current: Option<String> = self.redis.get(hold_key(&hold.seat))?;
        Ok(current.as_deref() == Some(hold.token.as_str()))
    }

    fn release(&mut self, hold: &Hold) -> bool {
        let released: RedisResult<i32> = Script::new(RELEASE_SCRIPT)
            .key(hold_key(&hold.seat))
            .arg(&hold.token)
            .invoke(&mut self.redis);
        match released {
            Ok(n) => n == 1,
            Err(e) => {
                eprintln!("     redis release failed, hold expires on its own: {e}");
                false
            }
        }
    }
}
