use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use redis::{Client, Commands, Connection, Msg, RedisResult};

/// How long a listener waits for a message before checking whether it should stop.
const POLL: Duration = Duration::from_millis(100);

/// The channel a room's messages travel on.
pub fn channel(room: &str) -> String {
    format!("chat:{room}")
}

/// Posts `text` to `room` as `user`. Returns how many subscribers received it.
///
/// PUBLISH doesn't store anything. If nobody is subscribed right now, the message is gone.
pub fn post(redis: &mut Connection, room: &str, user: &str, text: &str) -> RedisResult<usize> {
    redis.publish(channel(room), format!("{user}: {text}"))
}

/// One message as a listener received it.
pub struct Heard {
    pub channel: String,
    /// The pattern that matched, when the message came in through PSUBSCRIBE.
    pub pattern: Option<String>,
    pub text: String,
}

impl Heard {
    pub fn from_msg(msg: &Msg) -> RedisResult<Self> {
        Ok(Self {
            channel: msg.get_channel_name().to_string(),
            pattern: if msg.from_pattern() {
                Some(msg.get_pattern()?)
            } else {
                None
            },
            text: msg.get_payload()?,
        })
    }
}

/// What a listener subscribes to.
#[derive(Default)]
pub struct Subscription {
    /// Exact channel names, for SUBSCRIBE.
    pub channels: Vec<String>,
    /// Glob patterns such as `chat:*`, for PSUBSCRIBE.
    pub patterns: Vec<String>,
}

/// A subscriber running on its own thread with its own connection.
///
/// A connection in subscribe mode can only (un)subscribe and ping, so a
/// listener can never share a connection with the code that publishes.
pub struct Listener {
    pub name: String,
    stop: Arc<AtomicBool>,
    handle: JoinHandle<RedisResult<Vec<Heard>>>,
}

impl Listener {
    /// Subscribes and starts listening. Returns once Redis has confirmed the
    /// subscription, so anything published after this is guaranteed to arrive.
    pub fn start(client: &Client, name: &str, sub: Subscription) -> RedisResult<Self> {
        let mut redis = client.get_connection()?;
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel();

        let handle = thread::spawn({
            let stop = Arc::clone(&stop);
            move || {
                let mut pubsub = redis.as_pubsub();
                // Both calls wait for Redis to confirm before returning
                let subscribed = (|| {
                    if !sub.channels.is_empty() {
                        pubsub.subscribe(&sub.channels)?;
                    }
                    if !sub.patterns.is_empty() {
                        pubsub.psubscribe(&sub.patterns)?;
                    }
                    pubsub.set_read_timeout(Some(POLL))
                })();
                let failed = subscribed.is_err();
                let _ = ready_tx.send(subscribed);
                if failed {
                    return Ok(Vec::new());
                }

                let mut heard = Vec::new();
                loop {
                    match pubsub.get_message() {
                        Ok(msg) => heard.push(Heard::from_msg(&msg)?),
                        // Only stop after a quiet poll, so messages already
                        // on their way are not dropped
                        Err(e) if e.is_timeout() => {
                            if stop.load(Ordering::Relaxed) {
                                return Ok(heard);
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        });

        ready_rx.recv().expect("listener thread exited before subscribing")?;
        Ok(Self {
            name: name.to_string(),
            stop,
            handle,
        })
    }

    /// Stops listening and returns every message received, in order.
    pub fn stop(self) -> RedisResult<Vec<Heard>> {
        self.stop.store(true, Ordering::Relaxed);
        self.handle.join().expect("listener thread panicked")
    }
}
