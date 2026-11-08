mod chat;

use std::time::Duration;

use redis::{Client, Connection, PubSub, RedisResult};

use chat::{Heard, Listener, Subscription, channel, post};

/// Posts a message and prints how many listeners received it.
fn say(redis: &mut Connection, room: &str, user: &str, text: &str) -> RedisResult<()> {
    let receivers = post(redis, room, user, text)?;
    println!(
        "  {user:<6} -> {:<12} {text:<28} reached {receivers}",
        channel(room)
    );
    Ok(())
}

fn print_heard(name: &str, heard: &[Heard]) {
    println!("  {name} heard {} message(s):", heard.len());
    for h in heard {
        match &h.pattern {
            Some(p) => println!("    [{}] {}  (via pattern {p})", h.channel, h.text),
            None => println!("    [{}] {}", h.channel, h.text),
        }
    }
}

/// Reads messages until none arrive within the read timeout.
fn drain(pubsub: &mut PubSub<'_>) -> RedisResult<Vec<Heard>> {
    let mut heard = Vec::new();
    loop {
        match pubsub.get_message() {
            Ok(msg) => heard.push(Heard::from_msg(&msg)?),
            Err(e) if e.is_timeout() => return Ok(heard),
            Err(e) => return Err(e),
        }
    }
}

fn stop_all(listeners: Vec<Listener>) -> RedisResult<()> {
    for l in listeners {
        let name = l.name.clone();
        print_heard(&name, &l.stop()?);
    }
    println!();
    Ok(())
}

fn subscribe_to(rooms: &[&str]) -> Subscription {
    Subscription {
        channels: rooms.iter().map(|r| channel(r)).collect(),
        ..Default::default()
    }
}

fn demo_rooms(client: &Client, redis: &mut Connection) -> RedisResult<()> {
    println!("== Rooms: each room is a channel; only its subscribers get its messages");
    let listeners = vec![
        Listener::start(client, "alice", subscribe_to(&["rust"]))?,
        Listener::start(client, "carol", subscribe_to(&["go"]))?,
        Listener::start(client, "dave", subscribe_to(&["rust", "go"]))?,
    ];

    say(redis, "rust", "bob", "anyone tried edition 2024?")?;
    say(redis, "go", "erin", "generics are nice now")?;
    say(redis, "python", "frank", "hello? anyone?")?;

    stop_all(listeners)
}

fn demo_patterns(client: &Client, redis: &mut Connection) -> RedisResult<()> {
    println!("== Patterns: a moderator uses PSUBSCRIBE chat:* to watch every room");
    let listeners = vec![
        Listener::start(client, "alice", subscribe_to(&["rust"]))?,
        Listener::start(
            client,
            "mod",
            Subscription {
                patterns: vec![channel("*")],
                ..Default::default()
            },
        )?,
    ];

    // PUBSUB shows who is listening, without subscribing
    let active: Vec<String> = redis::cmd("PUBSUB")
        .arg("CHANNELS")
        .arg(channel("*"))
        .query(redis)?;
    let numsub: Vec<(String, usize)> = redis::cmd("PUBSUB")
        .arg("NUMSUB")
        .arg(channel("rust"))
        .query(redis)?;
    let numpat: usize = redis::cmd("PUBSUB").arg("NUMPAT").query(redis)?;
    println!("  PUBSUB CHANNELS chat:*  -> {active:?}");
    println!("  PUBSUB NUMSUB chat:rust -> {numsub:?}  (pattern subscribers not counted)");
    println!("  PUBSUB NUMPAT           -> {numpat}");

    say(redis, "rust", "bob", "cargo just got faster")?;
    say(redis, "go", "erin", "go 1.30 is out")?;
    say(redis, "random", "spam", "BUY CHEAP CRYPTO")?;

    stop_all(listeners)
}

fn demo_fire_and_forget(client: &Client, redis: &mut Connection) -> RedisResult<()> {
    println!("== Fire and forget: nothing is stored, so late joiners miss old messages");
    say(redis, "rust", "bob", "meeting moved to 3pm")?;

    let late = Listener::start(client, "grace", subscribe_to(&["rust"]))?;
    println!("  (grace joins chat:rust)");
    say(redis, "rust", "bob", "see you all there")?;

    stop_all(vec![late])
}

fn demo_unsubscribe(client: &Client, redis: &mut Connection) -> RedisResult<()> {
    println!("== Unsubscribe: heidi leaves chat:go but stays in chat:rust");
    // No thread here: one connection subscribes, another publishes,
    // and this thread reads the subscriber's messages itself
    let mut sub_con = client.get_connection()?;
    let mut pubsub = sub_con.as_pubsub();
    pubsub.subscribe(&[channel("rust"), channel("go")])?;
    pubsub.set_read_timeout(Some(Duration::from_millis(200)))?;

    say(redis, "rust", "bob", "first")?;
    say(redis, "go", "erin", "first")?;
    print_heard("heidi (in both rooms)", &drain(&mut pubsub)?);

    pubsub.unsubscribe(channel("go"))?;

    say(redis, "rust", "bob", "second")?;
    say(redis, "go", "erin", "second")?;
    print_heard("heidi (after leaving chat:go)", &drain(&mut pubsub)?);
    println!();
    Ok(())
}

fn main() -> RedisResult<()> {
    let client = Client::open("redis://127.0.0.1:6379/")?;
    // Publishing uses an ordinary connection
    let mut redis = client.get_connection()?;

    demo_rooms(&client, &mut redis)?;
    demo_patterns(&client, &mut redis)?;
    demo_fire_and_forget(&client, &mut redis)?;
    demo_unsubscribe(&client, &mut redis)
}
