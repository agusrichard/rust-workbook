mod ledger;
mod projector;

use std::{sync::Barrier, thread, time::Instant};

use redis::{Commands, RedisResult};

use ledger::{Append, Expected, Ledger};
use projector::Projector;

const SNAPSHOT_EVENTS: u64 = 1000;

/// Deletes every key of this demo (the streams, snapshots, views and consumer group).
fn reset(redis: &mut redis::Connection) -> RedisResult<()> {
    let keys: Vec<String> = redis
        .scan_match::<_, String>("bank:*")?
        .collect::<RedisResult<_>>()?;
    if !keys.is_empty() {
        redis::cmd("DEL").arg(&keys).exec(redis)?;
    }
    Ok(())
}

/// Runs a command and prints what happened.
fn run(
    ledger: &mut Ledger,
    account: &str,
    label: &str,
    decide: impl Fn(&ledger::Account) -> Result<ledger::Event, ledger::Rejected>,
) -> RedisResult<()> {
    match ledger.execute(account, decide)? {
        Ok(version) => println!("  {label:<14} ok, now at v{version}"),
        Err(rejected) => println!("  {label:<14} {rejected} (no event written)"),
    }
    Ok(())
}

fn demo_replay(ledger: &mut Ledger) -> RedisResult<()> {
    println!("== 1. Events are the truth: alice's balance is never stored");
    run(ledger, "alice", "deposit 100", |a| a.deposit(100))?;
    run(ledger, "alice", "withdraw 30", |a| a.withdraw(30))?;
    run(ledger, "alice", "deposit 50", |a| a.deposit(50))?;
    run(ledger, "alice", "withdraw 500", |a| a.withdraw(500))?;

    println!("\n  The log (XRANGE bank:account:alice - +):");
    for (i, (id, event)) in ledger.history("alice")?.iter().enumerate() {
        println!("    v{}  {id:<16}  {event}", i + 1);
    }

    let now = ledger.load("alice")?;
    println!("\n  Replayed: balance {} at v{}", now.balance, now.version);
    print!("  Time travel:");
    for v in 1..=now.version {
        print!("  v{v} = {}", ledger.load_at("alice", v)?.balance);
    }
    println!("\n");
    Ok(())
}

/// Two writers load the account, both decide "withdraw 80 is fine", then both append.
/// The barrier makes sure both have loaded before either appends, so the race always happens.
fn withdraw_at_same_moment(client: &redis::Client, account: &str, check: bool) -> RedisResult<()> {
    let start_line = Barrier::new(2);
    thread::scope(|s| {
        let handles: Vec<_> = ["atm", "app"]
            .into_iter()
            .map(|who| {
                let start_line = &start_line;
                s.spawn(move || -> RedisResult<String> {
                    let mut ledger = Ledger::new(client.get_connection()?);
                    let seen = ledger.load(account)?;
                    let event = seen.withdraw(80).expect("both writers see 100");
                    start_line.wait();

                    let expected = if check {
                        Expected::Exact(seen.version)
                    } else {
                        Expected::Any
                    };
                    let saw = format!("{who}: saw {} at v{}", seen.balance, seen.version);
                    Ok(match ledger.append(account, expected, event)? {
                        Append::Done { version } => format!("{saw} -> withdrew 80, now v{version}"),
                        Append::Conflict { actual } => {
                            // Someone wrote first. Load again and decide again
                            let retry = match ledger.execute(account, |a| a.withdraw(80))? {
                                Ok(version) => format!("withdrew 80, now v{version}"),
                                Err(rejected) => rejected.to_string(),
                            };
                            format!(
                                "{saw} -> CONFLICT (log is at v{actual}) -> reloaded -> {retry}"
                            )
                        }
                    })
                })
            })
            .collect();
        for handle in handles {
            println!("    {}", handle.join().expect("writer thread panicked")?);
        }
        Ok(())
    })
}

fn demo_race(client: &redis::Client, ledger: &mut Ledger) -> RedisResult<()> {
    println!("== 2. Two writers withdraw 80 from 100 at the same moment");
    for (account, check) in [("carol", false), ("dave", true)] {
        ledger
            .execute(account, |a| a.deposit(100))?
            .expect("deposits are never rejected");
        let how = if check {
            "with an expected-version check"
        } else {
            "with no check (Expected::Any)"
        };
        println!("  {account}, {how}:");
        withdraw_at_same_moment(client, account, check)?;
        println!("    final balance: {}", ledger.load(account)?.balance);
    }
    println!();
    Ok(())
}

fn demo_snapshot(ledger: &mut Ledger) -> RedisResult<()> {
    println!("== 3. Snapshots: erin makes {SNAPSHOT_EVENTS} deposits of 1");
    for _ in 0..SNAPSHOT_EVENTS {
        ledger
            .execute("erin", |a| a.deposit(1))?
            .expect("deposits are never rejected");
    }

    let saved = ledger.save_snapshot("erin")?;
    println!(
        "  Snapshot saved: balance {} at v{}",
        saved.balance, saved.version
    );
    for _ in 0..3 {
        ledger
            .execute("erin", |a| a.deposit(1))?
            .expect("deposits are never rejected");
    }
    println!("  ...then 3 more deposits");

    let t = Instant::now();
    let full = ledger.load("erin")?;
    let full_time = t.elapsed();
    let t = Instant::now();
    let (fast, read) = ledger.load_with_snapshot("erin")?;
    let fast_time = t.elapsed();

    println!(
        "  Full replay:        balance {} at v{}, read {} events in {full_time:.1?}",
        full.balance, full.version, full.version
    );
    println!(
        "  Snapshot + replay:  balance {} at v{}, read {read} events in {fast_time:.1?}",
        fast.balance, fast.version
    );
    println!();
    Ok(())
}

fn demo_projection(client: &redis::Client, ledger: &mut Ledger) -> RedisResult<()> {
    println!("== 4. Projection: a consumer group keeps a balances read model");

    // First run: takes 5 events, applies 3, then "crashes" before acking any of them
    let mut first = Projector::new(client.get_connection()?, "projector-1");
    let batch = first.read_new(5)?;
    for entry in &batch[..3] {
        first.apply(entry)?;
    }
    println!(
        "  projector-1 took {} events, applied 3, then crashed before XACK",
        batch.len()
    );
    println!("  Pending (handed out, never acked): {}", first.pending()?);
    drop(first);

    // Restart under the same name: Redis hands back the unacked events first
    let mut restarted = Projector::new(client.get_connection()?, "projector-1");
    let (applied, skipped) = restarted.catch_up(100)?;
    println!(
        "  projector-1 restarted: applied {applied}, skipped {skipped} already applied, pending now {}",
        restarted.pending()?
    );

    println!("\n  {:<8} {:>12} {:>12}", "account", "read model", "replay");
    for account in ["alice", "carol", "dave", "erin"] {
        let view = restarted.balance(account)?;
        let truth = ledger.load(account)?.balance;
        let mark = if view == truth { "" } else { "  MISMATCH" };
        println!("  {account:<8} {view:>12} {truth:>12}{mark}");
    }
    Ok(())
}

fn main() -> RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:6379/")?;
    let mut redis = client.get_connection()?;
    reset(&mut redis)?;
    // Created before any event, so the projector sees the whole history
    Projector::setup(&mut redis)?;

    let mut ledger = Ledger::new(client.get_connection()?);
    demo_replay(&mut ledger)?;
    demo_race(&client, &mut ledger)?;
    demo_snapshot(&mut ledger)?;
    demo_projection(&client, &mut ledger)
}
