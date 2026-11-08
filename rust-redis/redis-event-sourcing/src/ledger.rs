use std::{collections::HashMap, fmt};

use redis::{
    Commands, Connection, RedisResult, Script,
    streams::{StreamId, StreamRangeReply},
};

/// Every event from every account, in the order they happened. Projections read this.
pub const ALL_EVENTS: &str = "bank:events";

/// Something that happened to an account. Events are facts: once written, never changed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Deposited { amount: u64 },
    Withdrawn { amount: u64 },
}

impl Event {
    fn kind(&self) -> &'static str {
        match self {
            Event::Deposited { .. } => "deposited",
            Event::Withdrawn { .. } => "withdrawn",
        }
    }

    fn amount(&self) -> u64 {
        match self {
            Event::Deposited { amount } | Event::Withdrawn { amount } => *amount,
        }
    }

    /// How this event changes the balance.
    pub fn delta(&self) -> i64 {
        match self {
            Event::Deposited { amount } => *amount as i64,
            Event::Withdrawn { amount } => -(*amount as i64),
        }
    }

    /// Reads an event back from a stream entry.
    pub fn from_entry(entry: &StreamId) -> Event {
        let amount = field(entry, "amount");
        match field::<String>(entry, "type").as_str() {
            "deposited" => Event::Deposited { amount },
            "withdrawn" => Event::Withdrawn { amount },
            other => panic!("unknown event type {other:?} in entry {}", entry.id),
        }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.kind(), self.amount())
    }
}

/// Reads a field from a stream entry. Every entry is written by `append.lua`, so a
/// missing field means the log is corrupt.
pub fn field<T: redis::FromRedisValue>(entry: &StreamId, name: &str) -> T {
    entry
        .get(name)
        .unwrap_or_else(|| panic!("entry {} has no valid {name:?} field", entry.id))
}

/// Why a command was refused. A refused command writes no event.
#[derive(Debug)]
pub enum Rejected {
    InsufficientFunds { balance: i64, wanted: u64 },
}

impl fmt::Display for Rejected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rejected::InsufficientFunds { balance, wanted } => {
                write!(f, "rejected: balance is {balance}, can't withdraw {wanted}")
            }
        }
    }
}

/// An account's current state. Never stored anywhere: always rebuilt from its events.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Account {
    pub balance: i64,
    /// How many events this state is built from.
    pub version: u64,
}

impl Account {
    pub fn apply(&mut self, event: Event) {
        self.balance += event.delta();
        self.version += 1;
    }

    // Commands look at the current state and decide which event to write, if any

    pub fn deposit(&self, amount: u64) -> Result<Event, Rejected> {
        Ok(Event::Deposited { amount })
    }

    pub fn withdraw(&self, amount: u64) -> Result<Event, Rejected> {
        if self.balance < amount as i64 {
            return Err(Rejected::InsufficientFunds {
                balance: self.balance,
                wanted: amount,
            });
        }
        Ok(Event::Withdrawn { amount })
    }
}

/// Which version the account must be at for an append to go through.
pub enum Expected {
    /// Only append if the account is still at this version.
    Exact(u64),
    /// Append no matter what. Unsafe when two writers race; here only for the demo.
    Any,
}

pub enum Append {
    Done {
        version: u64,
    },
    /// Someone else appended first. The account is now at `actual`.
    Conflict {
        actual: u64,
    },
}

pub struct Ledger {
    redis: Connection,
    append: Script,
}

impl Ledger {
    pub fn new(redis: Connection) -> Self {
        Self {
            redis,
            // Baked into the binary at compile time
            append: Script::new(include_str!("append.lua")),
        }
    }

    fn stream_key(account: &str) -> String {
        format!("bank:account:{account}")
    }

    fn snapshot_key(account: &str) -> String {
        format!("bank:snapshot:{account}")
    }

    /// Writes one event to the account's stream and to `ALL_EVENTS`, in one atomic step.
    pub fn append(
        &mut self,
        account: &str,
        expected: Expected,
        event: Event,
    ) -> RedisResult<Append> {
        let expected = match expected {
            Expected::Exact(v) => v.to_string(),
            Expected::Any => "any".to_string(),
        };
        let (ok, version): (u8, u64) = self
            .append
            .key(Self::stream_key(account))
            .key(ALL_EVENTS)
            .arg(expected)
            .arg(account)
            .arg(event.kind())
            .arg(event.amount())
            .invoke(&mut self.redis)?;
        Ok(if ok == 1 {
            Append::Done { version }
        } else {
            Append::Conflict { actual: version }
        })
    }

    /// Load, decide, append. If another writer got in between, load again and decide again.
    /// Returns the new version, or why the command was refused.
    pub fn execute(
        &mut self,
        account: &str,
        decide: impl Fn(&Account) -> Result<Event, Rejected>,
    ) -> RedisResult<Result<u64, Rejected>> {
        loop {
            let state = self.load(account)?;
            let event = match decide(&state) {
                Ok(event) => event,
                Err(rejected) => return Ok(Err(rejected)),
            };
            match self.append(account, Expected::Exact(state.version), event)? {
                Append::Done { version } => return Ok(Ok(version)),
                Append::Conflict { .. } => continue,
            }
        }
    }

    /// Every event of the account, oldest first, with its stream entry ID.
    pub fn history(&mut self, account: &str) -> RedisResult<Vec<(String, Event)>> {
        let reply: StreamRangeReply = self.redis.xrange_all(Self::stream_key(account))?;
        Ok(reply
            .ids
            .iter()
            .map(|entry| (entry.id.clone(), Event::from_entry(entry)))
            .collect())
    }

    /// Rebuilds the current state by replaying every event from the start.
    pub fn load(&mut self, account: &str) -> RedisResult<Account> {
        Ok(self.replay(account, Account::default(), "-")?.0)
    }

    /// Time travel: the state as it was right after event number `version`.
    pub fn load_at(&mut self, account: &str, version: u64) -> RedisResult<Account> {
        let reply: StreamRangeReply =
            self.redis
                .xrange_count(Self::stream_key(account), "-", "+", version as usize)?;
        let mut state = Account::default();
        for entry in &reply.ids {
            state.apply(Event::from_entry(entry));
        }
        Ok(state)
    }

    /// Applies the events from `start` on top of `state`.
    /// Returns the new state, the ID of the last event seen, and how many events were read.
    fn replay(
        &mut self,
        account: &str,
        mut state: Account,
        start: &str,
    ) -> RedisResult<(Account, Option<String>, usize)> {
        let reply: StreamRangeReply = self.redis.xrange(Self::stream_key(account), start, "+")?;
        for entry in &reply.ids {
            state.apply(Event::from_entry(entry));
        }
        let last_id = reply.ids.last().map(|e| e.id.clone());
        Ok((state, last_id, reply.ids.len()))
    }

    /// Loads the state from the latest snapshot plus only the events written after it.
    /// Returns the state and how many events had to be read.
    pub fn load_with_snapshot(&mut self, account: &str) -> RedisResult<(Account, usize)> {
        let (state, _, read) = self.load_from_snapshot(account)?;
        Ok((state, read))
    }

    fn load_from_snapshot(
        &mut self,
        account: &str,
    ) -> RedisResult<(Account, Option<String>, usize)> {
        let snap: HashMap<String, String> = self.redis.hgetall(Self::snapshot_key(account))?;
        let Some(last_id) = snap.get("last_id") else {
            // No snapshot yet: replay everything
            return self.replay(account, Account::default(), "-");
        };
        let state = Account {
            balance: snap["balance"]
                .parse()
                .expect("snapshot balance is a number"),
            version: snap["version"]
                .parse()
                .expect("snapshot version is a number"),
        };
        // "(" makes the start exclusive: everything strictly after the snapshot's last event
        let (state, newer_last_id, read) = self.replay(account, state, &format!("({last_id}"))?;
        Ok((state, newer_last_id.or(Some(last_id.clone())), read))
    }

    /// Saves the current state, so later loads can skip the events up to here.
    /// The snapshot is only a shortcut: delete it and `load` still gives the same answer.
    pub fn save_snapshot(&mut self, account: &str) -> RedisResult<Account> {
        let (state, last_id, _) = self.load_from_snapshot(account)?;
        if let Some(last_id) = last_id {
            let _: () = self.redis.hset_multiple(
                Self::snapshot_key(account),
                &[
                    ("balance", state.balance.to_string()),
                    ("version", state.version.to_string()),
                    ("last_id", last_id),
                ],
            )?;
        }
        Ok(state)
    }
}
