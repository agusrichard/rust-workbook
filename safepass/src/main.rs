use clap::{Args, Parser, Subcommand};
use pam::Authenticator;
use std::process;

mod storage;

#[derive(Parser)]
#[command(author, version, about)]
#[command(
    long_about = "A command-line tool to securely store and retrieve passwords, \
                        protected by your macOS user credentials."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args)]
struct AddCommand {
    /// The name of the service (e.g., google, github)
    #[arg(short, long)]
    service: String,
    /// The username for the service
    #[arg(short, long)]
    username: String,
}

#[derive(Args)]
struct GetCommand {
    /// The name of the service to retrieve
    #[arg(short, long)]
    service: String,
}

#[derive(Subcommand)]
#[command(verbatim_doc_comment)]
enum Commands {
    /// Adds a new password entry to the vault.
    ///
    /// The service name must be unique. You will be prompted for the master password
    /// and the new password for the service.
    ///
    /// EXAMPLES:
    ///    safepass add --service google --username user@gmail.com
    Add(AddCommand),
    /// Retrieves a password from the vault.
    ///
    /// You will be prompted for the master password to decrypt the entry.
    ///
    /// EXAMPLES:
    ///    safepass get --service google
    Get(GetCommand),
    /// Lists all services and usernames stored in the vault.
    List,
}

fn verify_user_password() -> String {
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
            println!("Wrong password");
        } else {
            return password;
        }
    }

    eprintln!("Retries has exceeded. Please try again later!");
    process::exit(1);
}

fn handle_add_command(add_cmd: AddCommand, master_password: &str) {
    println!("Adding password for service: {}", add_cmd.service);
    let password = match rpassword::prompt_password("Enter password for service: ") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to read password: {}", e);
            process::exit(1);
        }
    };

    let confirm = match rpassword::prompt_password("Confirm password: ") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to read password: {}", e);
            process::exit(1);
        }
    };

    if password != confirm {
        eprintln!("Passwords do not match!");
        process::exit(1);
    }

    match storage::save_entry(
        &add_cmd.service,
        &add_cmd.username,
        &password,
        master_password,
    ) {
        Ok(_) => println!("Password for {} added successfully.", add_cmd.service),
        Err(e) => {
            eprintln!("Failed to save entry: {}", e);
            process::exit(1);
        }
    }
}

fn handle_get_command(get_cmd: GetCommand, master_password: &str) {
    let entries = match storage::load_entries() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to load entries: {}", e);
            process::exit(1);
        }
    };

    let entry = match entries.into_iter().find(|e| e.service == get_cmd.service) {
        Some(e) => e,
        None => {
            eprintln!("Service '{}' not found.", get_cmd.service);
            process::exit(1);
        }
    };

    match entry.decrypt(master_password) {
        Ok(password) => {
            println!("Service: {}", entry.service);
            println!("Username: {}", entry.username);
            println!("Password: {}", password);
        }
        Err(e) => {
            eprintln!("Failed to decrypt password: {}", e);
            process::exit(1);
        }
    }
}

fn handle_list_command() {
    let entries = match storage::load_entries() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to load entries: {}", e);
            process::exit(1);
        }
    };

    if entries.is_empty() {
        println!("No entries found in the vault.");
        return;
    }

    println!("{:<20} {:<30}", "SERVICE", "USERNAME");
    println!("{}", "-".repeat(50));
    for entry in entries {
        println!("{:<20} {:<30}", entry.service, entry.username);
    }
}

fn main() {
    let cli = Cli::parse();

    let master_password = verify_user_password();

    match cli.command {
        Commands::Add(add_cmd) => handle_add_command(add_cmd, &master_password),
        Commands::Get(get_cmd) => handle_get_command(get_cmd, &master_password),
        Commands::List => handle_list_command(),
    };
}
