mod cache;
mod db;

use std::{
    sync::{Arc, Barrier},
    thread,
    time::Instant,
};

use cache::ProductService;
use db::FakeDb;

fn timed_get(service: &mut ProductService, id: u32) -> redis::RedisResult<()> {
    let start = Instant::now();
    let product = service.get(id)?;
    println!("  {product:?} in {:?}\n", start.elapsed());
    Ok(())
}

/// Many requests ask for the same uncached product at the same moment.
fn stampede(client: &redis::Client, db: &Arc<FakeDb>, id: u32) -> redis::RedisResult<()> {
    const THREADS: usize = 10;

    let barrier = Arc::new(Barrier::new(THREADS));
    let loads_before = db.find_calls();

    // Each thread needs its own connection. Open them all before any thread
    // starts, so a failed connection cannot leave the others stuck at the barrier.
    let mut handles = Vec::with_capacity(THREADS);
    for _ in 0..THREADS {
        let mut service = ProductService::new(Arc::clone(db), client.get_connection()?);
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier.wait();
            service.get(id).map(|_| ())
        }));
    }
    for handle in handles {
        handle.join().expect("thread panicked")?;
    }

    println!("  database loads: {}\n", db.find_calls() - loads_before);
    Ok(())
}

fn main() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:6379/")?;
    let db = Arc::new(FakeDb::new());
    let mut service = ProductService::new(Arc::clone(&db), client.get_connection()?);

    // Start from a cold cache so every run prints the same story
    for id in [1, 2, 99] {
        service.invalidate(id)?;
    }
    service.reset_stats()?;

    println!("1st read of product 1");
    timed_get(&mut service, 1)?;

    println!("2nd read of product 1");
    timed_get(&mut service, 1)?;

    println!("Update price of product 1, then read again");
    service.update_price(1, 399)?;
    timed_get(&mut service, 1)?;

    println!("Read a product that does not exist");
    timed_get(&mut service, 99)?;

    println!("Read a product that does not exist and after fill the cache");
    timed_get(&mut service, 99)?;

    println!("10 requests for product 2 at the same moment");
    stampede(&client, &db, 2)?;

    let (hits, misses) = service.stats()?;
    let hit_rate = 100.0 * hits as f64 / (hits + misses).max(1) as f64;
    println!("Hit rate: {hit_rate:.0}% ({hits} hits, {misses} misses)");

    Ok(())
}
