use clap::{Parser, Subcommand};
use pam::Authenticator;
use std::process;

#[derive(Parser)]
#[command(name = "safepass")]
#[command(about = "A secure password manager CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new password entry
    Add { service: String },
    /// Retrieve a password entry
    Get { service: String },
}

fn verify_user_password() {
    let username = whoami::username();

    const RETRIES: u32 = 3;
    for _ in 0..RETRIES {
        // Prompt for password securely (hidden input)
        let password =
            match rpassword::prompt_password(format!("Please enter password for {}: ", username)) {
                Ok(pass) => pass,
                Err(err) => {
                    eprintln!("Error reading password: {}", err);
                    process::exit(1);
                }
            };

        // Verify credentials using PAM
        let mut authenticator = match Authenticator::with_password("sudo") {
            Ok(auth) => auth,
            Err(err) => {
                eprintln!("Failed to initialize PAM: {}", err);
                process::exit(1);
            }
        };

        authenticator
            .get_handler()
            .set_credentials(&username, &password);

        if let Err(_) = authenticator.authenticate() {
            eprintln!("Wrong password");
        } else {
            return;
        }
    }

    eprintln!("Retries has exceeded. Please try again later!");
    process::exit(1);
}

fn main() {
    let cli = Cli::parse();

    verify_user_password();

    match &cli.command {
        Commands::Add { service } => {
            println!("Adding password for: {}", service);
        }
        Commands::Get { service } => {
            println!("Retrieving password for: {}", service);
        }
    }
}
