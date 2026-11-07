mod fleet;

use std::{
    collections::HashSet,
    sync::Barrier,
    thread::{self, sleep},
    time::Duration,
};

use fleet::{Fleet, Hit, Search};

// Monas, the National Monument in central Jakarta, as (longitude, latitude). Every rider stands here
const MONAS: (f64, f64) = (106.8272, -6.1754);
// The Earth radius Redis uses, so the distances we place drivers at match what Redis measures
const EARTH_RADIUS_KM: f64 = 6372.797560856;
const RADIUS_KM: f64 = 3.0;
const STALE_AFTER: Duration = Duration::from_secs(1);
// Longer than STALE_AFTER, so every heartbeat sent before the wait goes stale
const WAIT_FOR_STALE: Duration = Duration::from_millis(1_100);
const RACERS: [&str; 5] = ["rider1", "rider2", "rider3", "rider4", "rider5"];

// Where each driver starts, as (name, km east of Monas, km north of Monas)
const DRIVERS: [(&str, f64, f64); 7] = [
    ("budi", 0.4, 0.0),
    ("sari", 0.0, 1.0),
    ("andi", -1.2, -1.2),
    ("dewi", -2.5, 0.0),
    ("eko", 0.0, -2.9),
    ("fajar", 4.0, 0.0),
    ("gita", 15.0, -3.0),
];

/// The (longitude, latitude) of a point `east_km` east and `north_km` north of Monas.
/// Treats the ground as flat, which is accurate enough over a few km.
fn near_monas(east_km: f64, north_km: f64) -> (f64, f64) {
    let (lon, lat) = MONAS;
    let dlat = (north_km / EARTH_RADIUS_KM).to_degrees();
    // A degree of longitude gets shorter the further you are from the equator
    let dlon = (east_km / (EARTH_RADIUS_KM * lat.to_radians().cos())).to_degrees();
    (lon + dlon, lat + dlat)
}

/// Every driver on `map` reports where they are, except those in `skip`.
fn report_all(fleet: &mut Fleet, map: &[(&str, f64, f64)], skip: &[&str]) -> redis::RedisResult<()> {
    for &(name, east, north) in map {
        if skip.contains(&name) {
            continue;
        }
        let (lon, lat) = near_monas(east, north);
        fleet.report(name, lon, lat)?;
    }
    Ok(())
}

fn print_hits(hits: &[Hit]) {
    if hits.is_empty() {
        println!("  (nobody)");
    }
    for (i, hit) in hits.iter().enumerate() {
        println!("  #{:<2} {:<6} {:>6.3} km", i + 1, hit.driver, hit.km);
    }
}

fn print_search(search: &Search) {
    for (label, hits) in [
        ("available", &search.available),
        ("busy", &search.busy),
        ("offline", &search.offline),
    ] {
        let names: Vec<String> = hits
            .iter()
            .map(|hit| format!("{} ({:.1} km)", hit.driver, hit.km))
            .collect();
        let names = if names.is_empty() { "-".to_string() } else { names.join(", ") };
        println!("  {label:<9} {names}");
    }
}

/// Every racer tries to book a driver at Monas at the same moment.
fn race(client: &redis::Client) -> redis::RedisResult<Vec<(&'static str, Option<Hit>)>> {
    // Connect first, so a failed connection can't leave the others stuck at the barrier
    let connections = RACERS
        .iter()
        .map(|_| client.get_connection())
        .collect::<redis::RedisResult<Vec<_>>>()?;
    let start = Barrier::new(RACERS.len());
    thread::scope(|s| {
        let handles: Vec<_> = RACERS
            .iter()
            .zip(connections)
            .map(|(&rider, connection)| {
                let start = &start;
                s.spawn(move || {
                    let mut fleet = Fleet::new(connection, STALE_AFTER);
                    start.wait();
                    let (lon, lat) = MONAS;
                    fleet
                        .book(rider, lon, lat, RADIUS_KM)
                        .map(|hit| (rider, hit))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("rider thread panicked"))
            .collect()
    })
}

fn main() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:6379/")?;
    let mut fleet = Fleet::new(client.get_connection()?, STALE_AFTER);
    let (lon, lat) = MONAS;
    let mut map = DRIVERS.to_vec();

    // Start with an empty map so every run prints the same story
    fleet.reset()?;

    println!("{} drivers report in around Monas", map.len());
    report_all(&mut fleet, &map, &[])?;
    println!("  {} drivers on the map", fleet.drivers()?);
    println!();

    println!("Under the hood, each position is a single Sorted Set score");
    println!("  {:<6} {:>8}  {:<12} score", "driver", "from Monas", "geohash");
    for &(name, east, north) in &map {
        let geohash = fleet.geohash(name)?.unwrap_or_default();
        let score = fleet.score(name)?.unwrap_or_default();
        println!("  {name:<6} {:>7.1} km  {geohash:<12} {score}", east.hypot(north));
    }
    println!("  Everyone near Monas starts with qqgu and has a similar score; gita, 15 km out, starts with qquh");
    println!("  But eko starts with qqguw, not qqguy: close points on either side of a cell edge can differ early.");
    println!("  That's why GEOSEARCH checks the neighbouring cells too, not just the one you're in");
    println!();

    println!("Positions come back a little off, because Redis keeps only the geohash");
    let (sent_lon, sent_lat) = near_monas(DRIVERS[0].1, DRIVERS[0].2);
    if let Some((got_lon, got_lat)) = fleet.position("budi")? {
        let east_m = (got_lon - sent_lon).to_radians() * lat.to_radians().cos() * EARTH_RADIUS_KM * 1000.0;
        let north_m = (got_lat - sent_lat).to_radians() * EARTH_RADIUS_KM * 1000.0;
        println!("  sent   {sent_lon:.7}, {sent_lat:.7}");
        println!("  stored {got_lon:.7}, {got_lat:.7}");
        println!("  {:.2} m apart", east_m.hypot(north_m));
    }
    println!();

    println!("How far apart are budi and sari?");
    if let Some(km) = fleet.distance("budi", "sari")? {
        println!("  {km:.3} km (placed {:.3} km apart)", 0.4_f64.hypot(1.0));
    }
    println!("  budi to someone not on the map: {:?}", fleet.distance("budi", "nobody")?);
    println!();

    println!("Every driver within {RADIUS_KM} km of Monas, nearest first");
    print_hits(&fleet.within(lon, lat, RADIUS_KM, None)?);
    println!("  fajar (4 km) and gita (15 km) are outside the circle");
    println!();

    println!("COUNT 2 keeps only the nearest 2");
    print_hits(&fleet.within(lon, lat, RADIUS_KM, Some(2))?);
    println!();

    println!("COUNT 0 returns nobody, not a Redis error");
    print_hits(&fleet.within(lon, lat, RADIUS_KM, Some(0))?);
    println!();

    println!("Searching the middle of the Indian Ocean is empty, not an error");
    print_hits(&fleet.within(80.0, -20.0, RADIUS_KM, None)?);
    println!();

    println!("Drivers within 1.5 km of sari, searching from a member (FROMMEMBER)");
    print_hits(&fleet.around("sari", 1.5)?);
    println!();

    println!("Searching around someone not on the map");
    match fleet.around("nobody", 1.5) {
        Ok(hits) => print_hits(&hits),
        Err(e) => println!("  Redis error: {e}"),
    }
    println!();

    println!("A 3 x 3 km box, like the map on the rider's screen (BYBOX)");
    print_hits(&fleet.in_box(lon, lat, 3.0, 3.0)?);
    println!("  andi is 1.7 km away, yet inside: the box reaches further into its corners than a 1.5 km circle");
    println!();

    println!("Sending latitude first by mistake");
    let (drivers, heartbeats) = (fleet.drivers()?, fleet.heartbeats()?);
    match fleet.report("oops", lat, lon) {
        Ok(_) => println!("  accepted?!"),
        Err(e) => println!("  Redis error: {e}"),
    }
    println!(
        "  drivers {drivers} -> {}, heartbeats {heartbeats} -> {}: the script stopped before the heartbeat",
        fleet.drivers()?,
        fleet.heartbeats()?
    );
    println!("  Jakarta gets caught because no latitude is 106. Most swapped pairs are valid and silently land elsewhere");
    println!();

    println!("budi drives 1.6 km east");
    map[0] = ("budi", 2.0, 0.0);
    let (budi_lon, budi_lat) = near_monas(2.0, 0.0);
    let added = fleet.report("budi", budi_lon, budi_lat)?;
    println!(
        "  added as new: {added}. Still {} drivers, GEOADD moved budi instead of adding him twice",
        fleet.drivers()?
    );
    print_hits(&fleet.within(lon, lat, RADIUS_KM, None)?);
    println!();

    println!("dewi's app crashes. Everyone else keeps reporting");
    sleep(WAIT_FOR_STALE);
    report_all(&mut fleet, &map, &["dewi"])?;
    println!("  GEOSEARCH alone still finds her");
    print_hits(&fleet.within(lon, lat, RADIUS_KM, None)?);
    println!("  Checking heartbeats shows her as offline");
    print_search(&fleet.search(lon, lat, RADIUS_KM)?);
    println!();

    println!("The sweep takes stale drivers off the map");
    println!("  removed {:?}", fleet.sweep()?);
    println!("  {} drivers on the map", fleet.drivers()?);
    println!();

    println!("dewi's app reconnects");
    let (dewi_lon, dewi_lat) = near_monas(DRIVERS[3].1, DRIVERS[3].2);
    let added = fleet.report("dewi", dewi_lon, dewi_lat)?;
    println!("  added as new: {added}. Her first report puts her back on the map");
    println!();

    println!("rina books a ride");
    match fleet.book("rina", lon, lat, RADIUS_KM)? {
        Some(hit) => println!("  {} picks up rina, {:.1} km away", hit.driver, hit.km),
        None => println!("  no drivers nearby"),
    }
    let search = fleet.search(lon, lat, RADIUS_KM)?;
    print_search(&search);
    println!();

    println!("Why search() doesn't pass COUNT to Redis");
    if let Some(nearest) = fleet.within(lon, lat, RADIUS_KM, Some(1))?.first() {
        println!("  GEOSEARCH ... COUNT 1 returns {}, who is busy", nearest.driver);
    }
    if let Some(first) = search.available.first() {
        println!("  search() filters the whole circle first, so it finds {}", first.driver);
    }
    println!();

    println!(
        "{} riders book at the same moment, with {} drivers free",
        RACERS.len(),
        search.available.len()
    );
    let mut booked = HashSet::new();
    for (rider, hit) in race(&client)? {
        match hit {
            Some(hit) => {
                let confirmed = fleet.rider_of(&hit.driver)?;
                println!(
                    "  {rider} got {:<5} ({:.1} km), Redis says {} drives {}",
                    hit.driver,
                    hit.km,
                    hit.driver,
                    confirmed.as_deref().unwrap_or("nobody")
                );
                booked.insert(hit.driver);
            }
            None => println!("  {rider}: no drivers nearby"),
        }
    }
    println!("  {} drivers booked, none of them twice", booked.len());
    println!();

    println!("sari drops rina off");
    println!("  ride ended: {}", fleet.finish("sari")?);
    println!("  ending it again: {}", fleet.finish("sari")?);
    print_search(&fleet.search(lon, lat, RADIUS_KM)?);
    println!();

    println!("gita logs off");
    println!("  left: {}", fleet.leave("gita")?);
    println!("  leaving again: {}", fleet.leave("gita")?);
    println!("  {} drivers on the map", fleet.drivers()?);

    Ok(())
}
