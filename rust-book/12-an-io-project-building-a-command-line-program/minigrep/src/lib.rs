use std::error::Error;
use std::{env, fs, process};

pub struct Config {
    pub query: String,
    pub filename: String,
    pub ignore_case: bool
}

impl Config {
    fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("Not enough arguments");
        }
        let query = args.get(1).expect("Search query in the first position is expected").clone();
        let filename = args.get(2).expect("Filename argument in the second position is expected").clone();
        let ignore_case = env::var("IGNORE_CASE").is_ok();
        Ok(Self { query, filename, ignore_case })
    }
}

pub fn parse_config(args: &[String]) -> Config {
    Config::build(&args).unwrap_or_else(|e| {
        eprintln!("Problem when parsing {e}");
        process::exit(1);
    })
}

fn search<'a>(query: &str, contents: &'a str, ignore_case: bool) -> Vec<&'a str> {
    if ignore_case {
        contents.lines().filter(|&line| line.to_lowercase().contains(&query.to_lowercase())).collect()
    } else {
        contents.lines().filter(|&line| line.contains(&query)).collect()
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(config.filename)?;

    for line in search(&config.query, &contents, config.ignore_case) {
        eprintln!("line: {line}");
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_sensitive() {
        let query = "duct";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Duct tape.";

        assert_eq!(vec!["safe, fast, productive."], search(query, contents, false));
    }

    #[test]
    fn case_insensitive() {
        let query = "rUsT";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Trust me.";

        assert_eq!(
            vec!["Rust:", "Trust me."],
            search(query, contents, true)
        );
    }
}