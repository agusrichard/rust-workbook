use redis::{
    Commands, Connection, RedisResult,
    streams::{StreamId, StreamPendingReply, StreamReadOptions, StreamReadReply},
};

use crate::ledger::{ALL_EVENTS, Event, field};

/// The consumer group that keeps the balances read model up to date.
pub const GROUP: &str = "balances-view";

/// Turns the event log into a read model: one small hash per account holding its balance,
/// so reading a balance is one HGET instead of a replay.
pub struct Projector {
    redis: Connection,
    /// The consumer name. Redis remembers which events it handed to which consumer.
    name: String,
}

impl Projector {
    /// Creates the consumer group. "0" means it starts from the very first event.
    pub fn setup(redis: &mut Connection) -> RedisResult<()> {
        redis.xgroup_create_mkstream(ALL_EVENTS, GROUP, "0")
    }

    pub fn new(redis: Connection, name: &str) -> Self {
        Self {
            redis,
            name: name.to_string(),
        }
    }

    fn view_key(account: &str) -> String {
        format!("bank:view:{account}")
    }

    /// Reads up to `count` events after `id`.
    /// `id` = ">" asks for events never given to anyone in the group.
    /// Any other `id` re-reads events already given to this consumer but never acked.
    fn read(&mut self, id: &str, count: usize) -> RedisResult<Vec<StreamId>> {
        let options = StreamReadOptions::default()
            .group(GROUP, &self.name)
            .count(count);
        let reply: Option<StreamReadReply> =
            self.redis.xread_options(&[ALL_EVENTS], &[id], &options)?;
        Ok(reply
            .map(|r| r.keys.into_iter().flat_map(|k| k.ids).collect())
            .unwrap_or_default())
    }

    pub fn read_new(&mut self, count: usize) -> RedisResult<Vec<StreamId>> {
        self.read(">", count)
    }

    /// Applies one event to the read model. Returns false if it was already applied.
    ///
    /// Redis delivers an event again if it was never acked (at-least-once), so applying
    /// must be idempotent. The view remembers the last version it applied per account and
    /// skips anything at or below it. Fine for one projector; several projectors writing the
    /// same account would need this check-and-write in a Lua script.
    pub fn apply(&mut self, entry: &StreamId) -> RedisResult<bool> {
        let account: String = field(entry, "account");
        let version: u64 = field(entry, "version");
        let key = Self::view_key(&account);

        let applied: Option<u64> = self.redis.hget(&key, "version")?;
        if applied.is_some_and(|v| version <= v) {
            return Ok(false);
        }

        // MULTI/EXEC, so the balance and the version it belongs to always move together
        let _: () = redis::pipe()
            .atomic()
            .hincr(&key, "balance", Event::from_entry(entry).delta())
            .hset(&key, "version", version)
            .query(&mut self.redis)?;
        Ok(true)
    }

    /// Tells Redis these events are done, so they're never delivered again.
    pub fn ack(&mut self, entries: &[StreamId]) -> RedisResult<()> {
        if entries.is_empty() {
            return Ok(());
        }
        let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        let _: usize = self.redis.xack(ALL_EVENTS, GROUP, &ids)?;
        Ok(())
    }

    fn apply_and_ack(&mut self, entries: &[StreamId]) -> RedisResult<(usize, usize)> {
        let (mut applied, mut skipped) = (0, 0);
        for entry in entries {
            if self.apply(entry)? {
                applied += 1;
            } else {
                skipped += 1;
            }
        }
        self.ack(entries)?;
        Ok((applied, skipped))
    }

    /// What a projector does when it starts: first finish the events it was handed before
    /// it stopped, then follow new ones until there are none left.
    /// Returns how many events were applied and how many skipped as already applied.
    pub fn catch_up(&mut self, batch: usize) -> RedisResult<(usize, usize)> {
        let (mut applied, mut skipped) = (0, 0);

        // 1. Leftovers: delivered to this consumer earlier, never acked
        let mut cursor = "0".to_string();
        loop {
            let entries = self.read(&cursor, batch)?;
            let Some(last) = entries.last() else { break };
            cursor = last.id.clone();
            let (a, s) = self.apply_and_ack(&entries)?;
            applied += a;
            skipped += s;
        }

        // 2. New events
        loop {
            let entries = self.read_new(batch)?;
            if entries.is_empty() {
                break;
            }
            let (a, s) = self.apply_and_ack(&entries)?;
            applied += a;
            skipped += s;
        }
        Ok((applied, skipped))
    }

    /// How many events the group has handed out but not had acked yet.
    pub fn pending(&mut self) -> RedisResult<usize> {
        let reply: StreamPendingReply = self.redis.xpending(ALL_EVENTS, GROUP)?;
        Ok(reply.count())
    }

    pub fn balance(&mut self, account: &str) -> RedisResult<i64> {
        Ok(self
            .redis
            .hget::<_, _, Option<i64>>(Self::view_key(account), "balance")?
            .unwrap_or(0))
    }
}
