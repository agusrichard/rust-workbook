mod limiter;

use std::{
    sync::Barrier,
    thread::{self, sleep},
    time::Duration,
};

use redis::Commands;

use limiter::{Decision, FixedWindow, RateLimiter, SlidingWindowLog, SlidingWindowLua, now_millis};

const LIMIT: u64 = 5;
const WINDOW: Duration = Duration::from_secs(5);
const RACERS: usize = 20;
const RACE_ROUNDS: usize = 3;

/// Sends `n` requests as `client` and prints each decision. Returns the last one.
fn send(limiter: &mut dyn RateLimiter, client: &str, n: usize) -> redis::RedisResult<Decision> {
    let mut last = None;
    for i in 1..=n {
        let d = limiter.hit(client)?;
        if d.allowed {
            println!("  {client:<6} #{i:<2} ALLOWED  ({} left)", d.remaining);
        } else {
            println!(
                "  {client:<6} #{i:<2} BLOCKED  (retry in {:.1?})",
                d.retry_after
            );
        }
        last = Some(d);
    }
    Ok(last.expect("n > 0"))
}

/// Sends `n` requests as `client` without printing them. Returns how many got through.
fn burst(limiter: &mut dyn RateLimiter, client: &str, n: usize) -> redis::RedisResult<usize> {
    let mut allowed = 0;
    for _ in 0..n {
        if limiter.hit(client)?.allowed {
            allowed += 1;
        }
    }
    Ok(allowed)
}

/// `RACERS` threads, each with its own limiter and connection, send one request as `who`
/// at the same moment. Returns how many got through.
fn race<L: RateLimiter + Send>(
    client: &redis::Client,
    make: impl Fn(redis::Connection) -> L,
    who: &str,
) -> redis::RedisResult<usize> {
    // Connect up front, so connecting doesn't spread the threads' start times apart
    let limiters = (0..RACERS)
        .map(|_| Ok(make(client.get_connection()?)))
        .collect::<redis::RedisResult<Vec<_>>>()?;
    // Holds every thread at the line until all of them are there
    let start_line = Barrier::new(RACERS);

    thread::scope(|s| {
        let handles: Vec<_> = limiters
            .into_iter()
            .map(|mut limiter| {
                let start_line = &start_line;
                s.spawn(move || {
                    start_line.wait();
                    limiter.hit(who).map(|d| d.allowed)
                })
            })
            .collect();

        let mut allowed = 0;
        for handle in handles {
            if handle.join().expect("racer thread panicked")? {
                allowed += 1;
            }
        }
        Ok(allowed)
    })
}

/// Deletes every limiter key so each run starts from scratch.
fn reset(redis: &mut redis::Connection) -> redis::RedisResult<()> {
    let keys: Vec<String> = redis
        .scan_match::<_, String>("rate:*")?
        .collect::<redis::RedisResult<_>>()?;
    if !keys.is_empty() {
        redis::cmd("DEL").arg(&keys).exec(redis)?;
    }
    Ok(())
}

/// Sleeps until `lead` before the next fixed-window boundary.
fn sleep_until_before_boundary(lead: Duration) {
    let window_ms = WINDOW.as_millis() as u64;
    let lead_ms = lead.as_millis() as u64;
    let mut until_boundary = window_ms - now_millis() % window_ms;
    if until_boundary < lead_ms {
        until_boundary += window_ms;
    }
    sleep(Duration::from_millis(until_boundary - lead_ms));
}

fn demo_basics(name: &str, limiter: &mut dyn RateLimiter) -> redis::RedisResult<()> {
    println!("== {name}: alice bursts 7 requests (limit {LIMIT} per {WINDOW:?})");
    let last = send(limiter, "alice", 7)?;

    println!("\n== {name}: bob is counted separately");
    send(limiter, "bob", 3)?;

    println!(
        "\n== {name}: waiting {:.1?} until alice is allowed again",
        last.retry_after
    );
    sleep(last.retry_after);
    send(limiter, "alice", 2)?;
    println!();
    Ok(())
}

fn demo_boundary(limiters: &mut [(&str, &mut dyn RateLimiter)]) -> redis::RedisResult<()> {
    let n = LIMIT as usize;
    println!("== Boundary burst: mallory sends {n}, waits 1s, sends {n} more");
    sleep_until_before_boundary(Duration::from_millis(500));

    // Burst 1 lands 0.5s before a fixed-window boundary...
    let mut first = Vec::new();
    for (_, limiter) in limiters.iter_mut() {
        first.push(burst(&mut **limiter, "mallory", n)?);
    }
    sleep(Duration::from_secs(1));
    // ...and burst 2 lands 0.5s after it, in a brand-new fixed window
    let mut second = Vec::new();
    for (_, limiter) in limiters.iter_mut() {
        second.push(burst(&mut **limiter, "mallory", n)?);
    }

    println!(
        "  {:<10} {:>8} {:>8} {:>14}",
        "", "burst 1", "burst 2", "allowed in 1s"
    );
    for (i, (name, _)) in limiters.iter().enumerate() {
        let (a, b) = (first[i], second[i]);
        println!("  {name:<10} {a:>6}/{n} {b:>6}/{n} {:>14}", a + b);
    }
    println!();
    Ok(())
}

fn demo_race(client: &redis::Client) -> redis::RedisResult<()> {
    println!("== Race: {RACERS} threads send 1 request each at the same moment (limit {LIMIT})");
    println!(
        "  {:<6} {:>16} {:>11}",
        "round", "two round trips", "lua script"
    );
    for round in 1..=RACE_ROUNDS {
        // A fresh client each round, so earlier rounds don't fill the window
        let who = format!("racer{round}");
        let racy = race(client, |c| SlidingWindowLog::new(c, LIMIT, WINDOW), &who)?;
        let atomic = race(client, |c| SlidingWindowLua::new(c, LIMIT, WINDOW), &who)?;
        println!("  {round:<6} {racy:>13}/{RACERS} {atomic:>8}/{RACERS}");
    }
    Ok(())
}

fn main() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:6379/")?;
    reset(&mut client.get_connection()?)?;

    let mut fixed = FixedWindow::new(client.get_connection()?, LIMIT, WINDOW);
    let mut sliding = SlidingWindowLog::new(client.get_connection()?, LIMIT, WINDOW);
    let mut lua = SlidingWindowLua::new(client.get_connection()?, LIMIT, WINDOW);

    demo_basics("fixed", &mut fixed)?;
    demo_basics("sliding", &mut sliding)?;
    demo_boundary(&mut [
        ("fixed", &mut fixed),
        ("sliding", &mut sliding),
        ("lua", &mut lua),
    ])?;
    demo_race(&client)
}
