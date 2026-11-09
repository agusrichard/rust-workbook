# Event Sourcing with Redis

## The idea

Don't store the **current state**. Store **what happened**, then work out the state from it.

```
Normal app:      balance = 120

Event sourced:   deposited 100 → withdrawn 30 → deposited 50
                 balance = 0 + 100 − 30 + 50 = 120
```

Events are facts in the past tense (`Deposited`, `Withdrawn`). You only add new ones. You never edit or delete them.

What you get:
- **Full history.** You can see why the balance is what it is.
- **Time travel.** Replay only the first N events to get the state at that point.
- **New views for free.** Replay the same events into a different shape at any time.

## Why Redis Streams fit

A Redis Stream is an append-only log, which is exactly what an event store is.

| Need | Redis command |
|---|---|
| Write an event | `XADD bank:account:alice * type deposited amount 100` |
| Read all events | `XRANGE bank:account:alice - +` |
| Read the first N events (time travel) | `XRANGE key - + COUNT N` |
| Read events after an ID | `XRANGE key (1794147696960-0 +` (`(` = exclusive) |
| Version of an account | `XLEN key` (the number of events) |

## The 4 building blocks in this project

### 1. Command → Event → State
```
command "withdraw 30"  →  check the rules against the current state  →  event "withdrawn 30"
```
A command can be **rejected**, for example if there isn't enough money. A rejected command writes **no event**. State is just `events.fold(apply)`.

### 2. Optimistic concurrency (expected version)
Two ATMs both load `balance 100 @ v1` and both decide "withdraw 80 is OK".
- **No check:** both events are written, so the balance is **−60**. 💥
- **Check:** each append says "only if the account is still at v1". The second append sees v2, gets a **conflict**, reloads, and is rejected. The balance is **20**. ✅

The check and the write must happen as one step, so they run together in a **Lua script** ([append.lua](src/append.lua)):
```lua
if XLEN(stream) ~= expected then return conflict end
XADD ...
```

### 3. Snapshots
Replaying 1,000,000 events on every load is slow. Every so often, save the state:
```
HSET bank:snapshot:erin balance 1000 version 1000 last_id <id>
```
On load, start from the snapshot and replay only `XRANGE key (last_id +`.
In the demo: **1003 events in 27ms** became **3 events in 1ms**.
A snapshot is only a cache. If you delete it, a full replay gives the same answer.

### 4. Projections (read models) with consumer groups
Replaying to answer "what's the balance?" is wasteful. A **projector** follows a stream of all events and keeps a ready-made answer:
```
bank:events  ──XREADGROUP──▶  projector  ──HINCRBY──▶  bank:view:alice {balance: 120}
                                  └──XACK──▶ "done"
```
- **Consumer group:** Redis remembers which events each consumer has received.
- **XACK:** marks an event as done. Events that were never acked stay **pending**.
- **Crash:** the restarted consumer reads its pending events first (`XREADGROUP ... 0`) and then new ones (`>`).
- Because of this, an event can arrive **twice** (at-least-once delivery). Keep the projection **idempotent**: store the last applied version and skip anything at or below it.

## Write side vs read side (CQRS)

```
          write                                   read
command ──▶ Lua append ──▶ bank:account:*   bank:view:* ◀── HGET (fast)
                     └──▶ bank:events ──▶ projector ──┘
```
The write side protects the rules. The read side is fast to query. They are linked only by events. This means the read side is **eventually consistent**: it can be a few milliseconds behind.

## Keys

| Key | Type | Holds |
|---|---|---|
| `bank:account:{id}` | Stream | One account's events: `version`, `type`, `amount` |
| `bank:events` | Stream | Every account's events: `account`, `version`, `type`, `amount` |
| `bank:snapshot:{id}` | Hash | `balance`, `version`, `last_id` (the last event included) |
| `bank:view:{id}` | Hash | Read model: `balance`, `version` (last event applied) |
| group `balances-view` on `bank:events` | Consumer group | Which events each projector has received and acked |

## Logic flow

### Big picture
```
 deposit / withdraw
        │
        ▼
 ┌─────────────┐  XRANGE   ┌──────────────────┐
 │ 1. load     │◀──────────│ bank:account:id  │◀──┐
 └─────┬───────┘           └──────────────────┘   │
       ▼ state (balance, version)                 │ XADD
 ┌─────────────┐                                  │
 │ 2. decide   │── rule broken ──▶ Rejected       │
 └─────┬───────┘   (no event)                     │
       ▼ event                                    │
 ┌─────────────┐  EVALSHA append.lua              │
 │ 3. append   │──────────────────────────────────┤
 └─────┬───────┘                                  │ XADD
       │ conflict? ──▶ back to 1. load            ▼
       ▼ ok                              ┌──────────────┐
    new version                          │ bank:events  │
                                         └──────┬───────┘
                                                │ XREADGROUP
                                                ▼
                                   ┌─────────────────────────┐ HINCRBY/HSET
                                   │ projector (idempotent)  │────────────▶ bank:view:id
                                   └───────────┬─────────────┘
                                               │ XACK
                                               ▼ done
```

### Write a command ([`Ledger::execute`](src/ledger.rs))
1. **Load:** `XRANGE bank:account:alice - +`, then apply each event: `balance += delta`, `version += 1`.
2. **Decide:** `account.withdraw(30)` returns an event, or `Rejected` (stop here and write nothing).
3. **Append:** run `append.lua` with the version seen in step 1.
   - Script: `XLEN` == expected? → `XADD` to the account stream **and** to `bank:events` → return `{1, new_version}`.
   - Otherwise → return `{0, current_version}`.
4. **Conflict?** Go back to step 1 and decide again with fresh state.

### Read the state
- **Now:** `XRANGE key - +` → replay everything.
- **Time travel:** `XRANGE key - + COUNT N` → replay the first N events only.
- **With a snapshot** ([`load_with_snapshot`](src/ledger.rs)):
  1. `HGETALL bank:snapshot:erin`. If it's empty, do a full replay.
  2. Otherwise start from the snapshot's `balance`/`version`.
  3. `XRANGE key (last_id +` → replay only the newer events.
- **Save a snapshot:** load as above, then `HMSET` the state plus the last event ID.

### Projector ([`Projector::catch_up`](src/projector.rs))
1. **Leftovers first:** `XREADGROUP ... STREAMS bank:events 0` returns events this consumer received but never acked. Repeat from the last ID until the result is empty.
2. **Then new events:** `XREADGROUP ... STREAMS bank:events >` in batches until empty.
3. **For each event:**
   - `HGET bank:view:{acct} version`. If event version ≤ stored → **skip** (already applied).
   - Else `MULTI` → `HINCRBY balance delta` + `HSET version v` → `EXEC`.
4. **`XACK`** the batch so the events are never delivered again.

### Crash scenario (demo 4)
```
read 5 (>) → apply 3 → 💥 no XACK      XPENDING = 5
restart    → read pending (0) → 5 back → 3 skipped, 2 applied → XACK
           → read new (>) → apply rest → XACK      XPENDING = 0
```

## Commands used

These were captured with `redis-cli MONITOR` while the demo ran. `<id>` is a stream entry ID such as `1794147696959-0`.

**Setup**

| Command | Why |
|---|---|
| `SCAN 0 MATCH bank:*` + `DEL ...` | Wipe the previous run (streams, snapshots, views, group) |
| `XGROUP CREATE bank:events balances-view 0 MKSTREAM` | Create the projector's group, starting from the first event (`0`). `MKSTREAM` creates the stream if it doesn't exist yet |

**Write side**

| Command | Why |
|---|---|
| `EVALSHA <sha> 2 bank:account:alice bank:events 1 alice withdrawn 30` | Run `append.lua`: 2 keys, then expected version, account, type, amount. Use `any` instead of a version to skip the check (the unsafe demo) |
| `SCRIPT LOAD` | Only if Redis replies `NOSCRIPT` (script not cached yet). redis-rs loads it and retries |
| ↳ `XLEN bank:account:alice` | Inside the script: current version |
| ↳ `XADD bank:account:alice * version 2 type withdrawn amount 30` | Inside the script: the event, on the account stream (`*` = Redis picks the ID) |
| ↳ `XADD bank:events * account alice version 2 type withdrawn amount 30` | Inside the script: the same event, on the global stream |

**Read side (rebuilding state)**

| Command | Why |
|---|---|
| `XRANGE bank:account:alice - +` | All events → current state |
| `XRANGE bank:account:alice - + COUNT 2` | First 2 events → state at v2 (time travel) |
| `HGETALL bank:snapshot:erin` | Load the snapshot (empty = none yet) |
| `HMSET bank:snapshot:erin balance 1000 version 1000 last_id <id>` | Save the snapshot. redis-rs's `hset_multiple` sends `HMSET`; a multi-field `HSET` does the same |
| `XRANGE bank:account:erin (<id> +` | Only the events after the snapshot (`(` = exclusive) |

**Projection**

| Command | Why |
|---|---|
| `XREADGROUP GROUP balances-view projector-1 COUNT 5 STREAMS bank:events >` | Get up to 5 events never given to anyone in the group |
| `XREADGROUP GROUP balances-view projector-1 COUNT 100 STREAMS bank:events 0` | Get this consumer's unacked events (after a crash). Then repeat with the last `<id>` instead of `0` |
| `HGET bank:view:alice version` | Idempotency check: was this event already applied? |
| `MULTI` / `HINCRBY bank:view:alice balance -30` / `HSET bank:view:alice version 2` / `EXEC` | Update the balance and its version together |
| `XACK bank:events balances-view <id> ...` | Mark the batch as done |
| `XPENDING bank:events balances-view` | How many events were handed out but never acked |
| `HGET bank:view:alice balance` | Query the read model: one O(1) lookup, no replay |

## Gotchas
- **Never trim** an account stream (`XTRIM`/`MAXLEN`). The events are the data, and here `XLEN` is the version.
- **Redis is in memory.** Turn on AOF (`appendonly yes`, as this repo's docker-compose does) or the events can be lost.
- **Redis Cluster:** a Lua script's keys must be in the same hash slot. That breaks for "account stream + global stream". Use hash tags, or publish to the global stream separately (the outbox pattern).
- **Events are forever.** Design them carefully and version them (`deposited_v2`) instead of changing old ones.

## Run it
```bash
docker compose up -d          # from rust-redis/
cd redis-event-sourcing && cargo run
```
