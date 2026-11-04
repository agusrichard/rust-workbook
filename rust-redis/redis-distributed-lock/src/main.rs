mod box_office;
mod db;

use std::{
    sync::{Arc, Barrier},
    thread::{self, sleep},
    time::Duration,
};

use box_office::{BoxOffice, CheckoutError, HOLD_TTL, Hold, HoldResult, SeatStatus};
use db::FakeDb;

const SEATS: [&str; 5] = ["A1", "A2", "A3", "A4", "A5"];
const FAST_PAYMENT: Duration = Duration::from_millis(200);
// Longer than HOLD_TTL, so the hold runs out while the card is still charging
const SLOW_PAYMENT: Duration = Duration::from_millis(1_500);
// Long enough for any hold taken just before to run out
const WAIT_FOR_EXPIRY: Duration = Duration::from_millis(1_100);

/// Prints what happened when a fan clicked a seat. Returns the hold if they got it.
fn report(seat: &str, fan: &str, result: HoldResult) -> Option<Hold> {
    match result {
        HoldResult::Held(hold) => {
            println!("  {fan} holds {seat} for {HOLD_TTL:?}");
            Some(hold)
        }
        HoldResult::HeldBy(other) => {
            println!("  {fan}: {seat} is held by {other}, try another seat");
            None
        }
        HoldResult::Sold => {
            println!("  {fan}: {seat} is sold");
            None
        }
    }
}

fn hold(office: &mut BoxOffice, seat: &str, fan: &str) -> redis::RedisResult<Option<Hold>> {
    let result = office.hold(seat, fan)?;
    Ok(report(seat, fan, result))
}

fn pay(office: &mut BoxOffice, hold: &Hold, payment_time: Duration) -> redis::RedisResult<()> {
    let (fan, seat) = (&hold.fan, &hold.seat);
    match office.checkout(hold, payment_time) {
        Ok(()) => println!("  {fan} booked {seat}"),
        Err(CheckoutError::HoldExpired) => {
            println!("  {fan}: hold on {seat} expired, card not charged")
        }
        Err(CheckoutError::SeatSold) => {
            println!("  {fan}: {seat} was sold to someone else, card refunded")
        }
        Err(CheckoutError::Redis(e)) => return Err(e),
    }
    Ok(())
}

/// Every fan clicks the same seat at the same moment. Returns the winner's hold.
fn race(
    client: &redis::Client,
    db: &Arc<FakeDb>,
    seat: &str,
    fans: &[&str],
) -> redis::RedisResult<Option<Hold>> {
    let barrier = Arc::new(Barrier::new(fans.len()));

    // Each thread needs its own connection. Open them all before any thread
    // starts, so a failed connection cannot leave the others stuck at the barrier.
    let mut handles = Vec::with_capacity(fans.len());
    for fan in fans {
        let mut office = BoxOffice::new(Arc::clone(db), client.get_connection()?);
        let barrier = Arc::clone(&barrier);
        let (seat, fan) = (seat.to_string(), fan.to_string());
        handles.push(thread::spawn(move || {
            barrier.wait();
            office.hold(&seat, &fan).map(|result| (fan, result))
        }));
    }

    let mut winner = None;
    for handle in handles {
        let (fan, result) = handle.join().expect("thread panicked")?;
        if let Some(hold) = report(seat, &fan, result) {
            winner = Some(hold);
        }
    }
    Ok(winner)
}

fn main() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:6379/")?;
    let db = Arc::new(FakeDb::new(&SEATS));
    let mut office = BoxOffice::new(Arc::clone(&db), client.get_connection()?);

    // Start with no holds so every run prints the same story
    office.clear_holds()?;

    println!("Two fans click A1 at the same moment");
    if let Some(winner) = race(&client, &db, "A1", &["alice", "bob"])? {
        pay(&mut office, &winner, FAST_PAYMENT)?;
    }
    println!();

    println!("Carol holds A2, then walks away without paying");
    let carol = hold(&mut office, "A2", "carol")?;
    hold(&mut office, "A2", "dave")?;
    sleep(WAIT_FOR_EXPIRY);
    println!("  ...the hold expires, so A2 is free again");
    let dave = hold(&mut office, "A2", "dave")?;
    if let Some(carol) = carol {
        println!("  carol comes back and tries to pay");
        pay(&mut office, &carol, FAST_PAYMENT)?;
        // The release script checks the token, so this cannot delete dave's hold
        let released = office.cancel(&carol);
        println!("  carol cancels, released anything: {released}");
    }
    if let Some(dave) = dave {
        pay(&mut office, &dave, FAST_PAYMENT)?;
    }
    println!();

    println!("Erin holds A3, but the card takes {SLOW_PAYMENT:?} to charge");
    let erin_paying = match hold(&mut office, "A3", "erin")? {
        Some(erin) => {
            let mut office = BoxOffice::new(Arc::clone(&db), client.get_connection()?);
            Some(thread::spawn(move || pay(&mut office, &erin, SLOW_PAYMENT)))
        }
        None => None,
    };
    sleep(WAIT_FOR_EXPIRY);
    println!("  ...the hold expires while erin is still paying");
    if let Some(frank) = hold(&mut office, "A3", "frank")? {
        pay(&mut office, &frank, FAST_PAYMENT)?;
    }
    if let Some(erin_paying) = erin_paying {
        erin_paying.join().expect("thread panicked")?;
    }
    println!();

    println!("10 fans click A4 at the same moment");
    let names: Vec<String> = (1..=10).map(|i| format!("fan{i}")).collect();
    let fans: Vec<&str> = names.iter().map(String::as_str).collect();
    race(&client, &db, "A4", &fans)?;
    println!();

    println!("Seat map");
    for (seat, status) in office.seat_map()? {
        match status {
            SeatStatus::Available => println!("  {seat}  available"),
            SeatStatus::Held { fan } => println!("  {seat}  held by {fan}"),
            SeatStatus::Sold { fan } => println!("  {seat}  sold to {fan}"),
        }
    }

    Ok(())
}
