# Redis Pub/Sub: Live Chat Rooms

## The idea

Pub/Sub works like a radio. A **publisher** broadcasts a message on a **channel**, and every **subscriber** tuned in *right now* hears it. Redis just forwards the message. It does not store it.

```
bob ──PUBLISH chat:rust "hi"──►  Redis ──► alice (SUBSCRIBE chat:rust)
                                       └─► mod   (PSUBSCRIBE chat:*)
```

In this project, each chat room is one channel: `chat:rust`, `chat:go`, …

## Commands

| Command | What it does | Used for |
|---|---|---|
| `PUBLISH ch msg` | Sends `msg` to everyone on `ch`. Returns how many got it | Posting to a room |
| `SUBSCRIBE ch [ch …]` | Listens to exact channel names | Joining rooms |
| `PSUBSCRIBE pattern` | Listens to every channel that matches a glob such as `chat:*` | Moderator watching all rooms |
| `UNSUBSCRIBE ch` / `PUNSUBSCRIBE p` | Stops listening | Leaving a room |
| `PUBSUB CHANNELS [pattern]` | Lists channels that have at least one subscriber | Finding active rooms |
| `PUBSUB NUMSUB ch` | Counts the subscribers on a channel (pattern subscribers not included) | Room headcount |
| `PUBSUB NUMPAT` | Counts the active pattern subscriptions | Monitoring |

## Five things to remember

1. **Fire and forget.** If nobody is subscribed, `PUBLISH` returns `0` and the message is gone. Late joiners never see old messages (see the `meeting moved to 3pm` demo).
2. **At most once.** If a subscriber disconnects, or is too slow and Redis drops it, its messages are lost. There are no acks and no retries.
3. **A subscribed connection is dedicated.** In subscribe mode, a connection can only (un)subscribe and `PING`. So use **one connection to publish and a separate one to subscribe**. That's why each `Listener` gets its own connection and thread.
4. **Patterns cost a little more.** Each `PUBLISH` checks every active pattern. A message can arrive twice if a client matches it through both a channel and a pattern.
5. **Channels are not keys.** They don't live in the keyspace, so a database number (`SELECT`) has no effect on them. Their names are free-form, but `prefix:name` keeps them organised.

## When to use it (and when not)

| Good fit ✅ | Use **Redis Streams** instead ❌ |
|---|---|
| Live chat, typing indicators | Messages must not be lost |
| Telling app servers to clear their caches | Consumers go offline and need to catch up |
| Live dashboards and notifications | Work queues that need acks and retries |

## Commands this project sends

| Where in the code | Command sent to Redis | Redis replies |
|---|---|---|
| `chat::post` | `PUBLISH chat:rust "bob: cargo just got faster"` | `(integer) 2`, the number of receivers |
| `Listener::start` | `SUBSCRIBE chat:rust chat:go` | One confirmation per channel: `subscribe chat:rust 1`, `subscribe chat:go 2` |
| `Listener::start` (mod) | `PSUBSCRIBE chat:*` | `psubscribe chat:* 1` |
| `demo_patterns` | `PUBSUB CHANNELS chat:*` | `["chat:rust"]` |
| `demo_patterns` | `PUBSUB NUMSUB chat:rust` | `chat:rust 1` |
| `demo_patterns` | `PUBSUB NUMPAT` | `(integer) 1` |
| `demo_unsubscribe` | `UNSUBSCRIBE chat:go` | `unsubscribe chat:go 1` (1 = channels still subscribed) |
| Dropping a `PubSub` | `UNSUBSCRIBE` + `PUNSUBSCRIBE` (no arguments, so everything) | The crate sends this for you, which returns the connection to normal mode |

Messages arrive at subscribers on their own, as **push messages**. They are not replies to a command:

```
message   chat:rust          "bob: cargo just got faster"     ← from SUBSCRIBE
pmessage  chat:*  chat:go    "erin: go 1.30 is out"           ← from PSUBSCRIBE (pattern, channel, payload)
```

`Heard::from_msg` reads these with `get_channel_name()`, `get_pattern()` (pmessage only) and `get_payload()`.

### Try it by hand with redis-cli

```bash
# terminal 1: join a room
redis-cli SUBSCRIBE chat:rust
# terminal 2: moderator watches every room
redis-cli PSUBSCRIBE 'chat:*'
# terminal 3: post a message
redis-cli PUBLISH chat:rust "bob: hi"     # → (integer) 2
redis-cli PUBLISH chat:go   "erin: hey"   # → (integer) 1 (only the moderator)
redis-cli PUBSUB NUMSUB chat:rust         # → chat:rust 1
```

## Logic flow

### A listener's life

```
main thread                     listener thread                       Redis
───────────                     ───────────────                       ─────
Listener::start
  ├─ get_connection()
  ├─ spawn ───────────────────► as_pubsub()
  │                             SUBSCRIBE chat:rust ────────────────► registers alice
  │                             ◄────────────────── confirmation ─────
  │                             set_read_timeout(100ms)
  │ ◄───── "ready" (mpsc) ───── 
  │   (safe to publish now)     loop {
post()                            get_message()  ← blocks for up to 100ms
  ├─ PUBLISH chat:rust ... ───────────────────────────────────────►  fans out
  │ ◄────────────────────────────────────────────── (integer) 1 ───
  │                               ◄─────────────── message ─────────
  │                               push into Vec<Heard>
stop()                            on timeout: is the stop flag set?
  ├─ set stop flag = true            no  → keep looping
  └─ join() ◄─── Vec<Heard> ─────    yes → return everything heard
                                  }
print_heard()                   (PubSub dropped → UNSUBSCRIBE + PUNSUBSCRIBE)
```

Two details make the demos reliable:

- **Wait for "ready" before publishing.** `subscribe()` only returns after Redis confirms. If main published earlier, the message could go out before the subscription exists, and it would be lost.
- **Stop only after a quiet poll.** Setting the stop flag doesn't end the loop straight away. The listener waits for a read timeout with nothing arriving, so messages already in flight still get collected.

### What each demo does

```
1. Rooms              start alice[rust], carol[go], dave[rust,go]
                      → PUBLISH rust (2), go (2), python (0)
                      → stop all, print what each heard

2. Patterns           start alice[rust], mod[chat:*]
                      → PUBSUB CHANNELS / NUMSUB / NUMPAT
                      → PUBLISH rust (2), go (1), random (1)
                      → mod hears all 3 as pmessages

3. Fire and forget    PUBLISH rust (0)    ← nobody listening, message gone
                      → start grace[rust]
                      → PUBLISH rust (1)  ← grace hears only this one

4. Unsubscribe        (no thread: one connection subscribes, another publishes)
                      SUBSCRIBE rust go → PUBLISH both → drain() → 2 heard
                      UNSUBSCRIBE go    → PUBLISH both → drain() → 1 heard
```

## Files

- [`src/chat.rs`](src/chat.rs): `post` (PUBLISH), `Listener` (SUBSCRIBE/PSUBSCRIBE on a thread), and `Heard` (a parsed message)
- [`src/main.rs`](src/main.rs): the four demos plus `drain`, which reads messages until none arrive within the timeout

Run it with `docker compose up -d` (from the repo root), then `cargo run`.
