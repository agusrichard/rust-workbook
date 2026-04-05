use std::{env, process};

use minigrep::{parse_config, run};

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = parse_config(&args);
    println!("query: {}", config.query);
    println!("filename: {}", config.filename);

    if let Err(e) = run(config) {
        println!("Application error: {e}");
        process::exit(1);
    }
}
