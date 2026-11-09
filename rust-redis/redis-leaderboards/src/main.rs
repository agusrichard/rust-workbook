mod board;

use std::{thread::sleep, time::Duration};

use board::{Aggregate, Board};
use chrono::{Datelike, Days};

const PLAYERS: usize = 20;
const RUNS_PER_PLAYER: usize = 3;
const MAX_SCORE: u64 = 1_000;
// Long enough for the TTL, counted in whole seconds, to tick down at least once
const WAIT_FOR_TTL_TICK: Duration = Duration::from_millis(1_100);

/// Prints the board as a numbered table, best first.
fn print_top(board: &mut Board, date: &str, n: isize) -> redis::RedisResult<()> {
    print_rows(&board.top(date, n)?);
    Ok(())
}

fn print_rows(rows: &[(String, u64)]) {
    if rows.is_empty() {
        println!("  (empty)");
    }
    for (i, (player, score)) in rows.iter().enumerate() {
        println!("  #{:<3} {player:<10} {score:>5}", i + 1);
    }
}

/// Prints the players around `player`, marking their own row.
fn print_around(
    board: &mut Board,
    date: &str,
    player: &str,
    radius: usize,
) -> redis::RedisResult<()> {
    let window = board.around(date, player, radius)?;
    if window.is_empty() {
        println!("  (not on the board)");
    }
    for (rank, name, score) in window {
        let marker = if name == player { ">" } else { " " };
        println!("{marker} #{:<3} {name:<10} {score:>5}", rank + 1);
    }
    Ok(())
}

fn main() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:6379/")?;
    let mut board = Board::new(client.get_connection()?);
    let now = chrono::Local::now().date_naive();
    let today = now.format("%Y-%m-%d").to_string();
    // ISO weeks start on Monday, e.g. 2026-W45
    let week = format!("{}-W{:02}", now.iso_week().year(), now.iso_week().week());
    let monday = now - Days::new(now.weekday().num_days_from_monday().into());
    let week_dates: Vec<String> = (0..7)
        .map(|day| (monday + Days::new(day)).format("%Y-%m-%d").to_string())
        .collect();

    // Start with empty boards so every run prints the same story
    for date in &week_dates {
        board.reset(date)?;
    }

    println!("Alice plays three runs, only her best one counts");
    for score in [300, 500, 200] {
        let best = board.submit(&today, "alice", score)?;
        println!("  alice scores {score:>4}, new best: {best}");
    }
    println!();

    println!("{PLAYERS} players play {RUNS_PER_PLAYER} runs each");
    for i in 1..=PLAYERS {
        let player = format!("player{i:02}");
        for _ in 0..RUNS_PER_PLAYER {
            board.submit(&today, &player, rand::random_range(0..=MAX_SCORE))?;
        }
    }
    println!();

    println!("Top 10");
    print_top(&mut board, &today, 10)?;
    println!();

    println!("Top 5 returns exactly 5 rows");
    print_top(&mut board, &today, 5)?;
    println!();

    println!("Top 0 returns nothing, not the whole board");
    print_top(&mut board, &today, 0)?;
    println!();

    println!("A day nobody played is an empty board, not an error");
    print_top(&mut board, "1999-01-01", 10)?;
    println!();

    let players = board.players(&today)?;
    println!("Where does alice stand?");
    match board.rank(&today, "alice")? {
        Some(rank) => println!("  alice is #{} of {players}", rank + 1),
        None => println!("  alice hasn't played today"),
    }
    print_around(&mut board, &today, "alice", 2)?;
    println!();

    let everyone = board.top(&today, players as isize)?;
    if let (Some((first, _)), Some((last, _))) = (everyone.first(), everyone.last()) {
        println!("Around #1, the window can't start above rank 0");
        print_around(&mut board, &today, first, 2)?;
        println!();

        println!("Around the last player, Redis returns whatever is left");
        print_around(&mut board, &today, last, 2)?;
        println!();
    }

    println!("Around someone who hasn't played");
    print_around(&mut board, &today, "nobody", 2)?;
    println!();

    println!("Daily boards delete themselves");
    let before = board.ttl(&today)?;
    println!(
        "  lb:{today} expires in {}d {}h",
        before / 86_400,
        before % 86_400 / 3_600
    );
    sleep(WAIT_FOR_TTL_TICK);
    board.submit(&today, "alice", 999)?;
    let after = board.ttl(&today)?;
    println!("  alice scores again, TTL went {before}s -> {after}s, not reset by EXPIRE NX");
    println!();

    println!("Earlier this week, each player showed up on about half the days");
    for date in week_dates.iter().filter(|date| **date < today) {
        for i in 1..=PLAYERS {
            if rand::random_bool(0.5) {
                let player = format!("player{i:02}");
                board.submit(date, &player, rand::random_range(0..=MAX_SCORE))?;
            }
        }
    }
    println!();

    let players = board.build_weekly(&week, &week_dates, Aggregate::Max)?;
    println!("Week {week}, best single run ({players} players)");
    print_rows(&board.top_weekly(&week, 5)?);
    println!();

    let players = board.build_weekly(&week, &week_dates, Aggregate::Sum)?;
    println!("Week {week}, all daily bests added up ({players} players)");
    print_rows(&board.top_weekly(&week, 5)?);

    Ok(())
}
