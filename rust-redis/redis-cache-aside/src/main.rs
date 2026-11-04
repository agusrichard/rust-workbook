mod cache;
mod db;

use std::time::Instant;

use cache::ProductService;
use db::FakeDb;

fn timed_get(service: &mut ProductService, id: u32) -> redis::RedisResult<()> {
    let start = Instant::now();
    let product = service.get(id)?;
    println!("  {product:?} in {:?}\n", start.elapsed());
    Ok(())
}

fn main() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:6379/")?;
    let mut service = ProductService::new(FakeDb::new(), client.get_connection()?);

    println!("1st read of product 1");
    timed_get(&mut service, 1)?;

    println!("2nd read of product 1");
    timed_get(&mut service, 1)?;

    println!("Update price of product 1, then read again");
    service.update_price(1, 399)?;
    timed_get(&mut service, 1)?;

    println!("Read a product that does not exist");
    timed_get(&mut service, 99)?;

    Ok(())
}
