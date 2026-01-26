use clap::{Args, Parser, Subcommand};
use cli_clipboard::{ClipboardContext, ClipboardProvider};
use magic_crypt::{MagicCrypt, MagicCryptTrait, SecureBit};
use rpassword::read_password;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::PathBuf;

fn get_vault_path() -> PathBuf {
    let mut path = dirs::home_dir().expect("Failed to get home directory");
    path.push(".safepass.json.enc");
    path
}

#[derive(Parser)]
#[command(name = "safepass")]
#[command(author = "Your Name <you@example.com>")]
#[command(version = "0.1.0")]
#[command(about = "A simple local password manager", long_about = "SafePass is a command-line interface (CLI) tool for securely storing and managing passwords in a local, encrypted file. It uses a master password to encrypt and decrypt the password vault.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
    /// Retrieves a password for a service and copies it to the clipboard.
    ///
    /// The password for the specified service will be copied to your system's clipboard.
    ///
    /// EXAMPLES:
    ///   safepass get --service google
    Get(GetCommand),
    /// Lists all services currently stored in the vault.
    ///
    /// This command will print a list of all service names, one per line.
    ///
    /// EXAMPLES:
    ///   safepass list
    List,
    /// Deletes a password entry from the vault.
    ///
    /// This will permanently remove the entry for the specified service.
    ///
    /// EXAMPLES:
    ///   safepass delete --service google
    Delete(DeleteCommand),
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
    /// The name of the service to retrieve the password for
    #[arg(short, long)]
    service: String,
}

#[derive(Args)]
struct DeleteCommand {
    /// The name of the service to delete
    #[arg(short, long)]
    service: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PasswordEntry {
    service: String,
    username: String,
    password: String,
}

fn get_master_password() -> String {
    print!("Enter master password: ");
    std::io::stdout().flush().unwrap();
    read_password().unwrap()
}

fn read_vault(master_password: &str) -> Result<Vec<PasswordEntry>, Box<dyn std::error::Error>> {
    let path = get_vault_path();
    let mut file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        Err(_) => return Ok(Vec::new()), // If file doesn't exist, return empty vault
    };

    let mut encrypted_data = String::new();
    file.read_to_string(&mut encrypted_data)?;

    if encrypted_data.is_empty() {
        return Ok(Vec::new());
    }

    let mc = MagicCrypt::new(master_password, SecureBit::Bit256, Option::<&[u8]>::None);
    let decrypted_string = mc.decrypt_base64_to_string(&encrypted_data)?;

    let entries: Vec<PasswordEntry> = serde_json::from_str(&decrypted_string)?;
    Ok(entries)
}

fn write_vault(
    master_password: &str,
    entries: &[PasswordEntry],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_vault_path();
    let mc = MagicCrypt::new(master_password, SecureBit::Bit256, Option::<&[u8]>::None);
    let json_string = serde_json::to_string(entries)?;
    let encrypted_string = mc.encrypt_str_to_base64(&json_string);

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(encrypted_string.as_bytes())?;
    Ok(())
}

fn handle_add_command(add_cmd: AddCommand) {
    let master_password = get_master_password();
    let mut entries = match read_vault(&master_password) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Error reading vault: {}", e);
            return;
        }
    };

    if entries.iter().any(|e| e.service == add_cmd.service) {
        eprintln!("Service '{}' already exists.", add_cmd.service);
        return;
    }

    print!("Enter password for {}: ", add_cmd.service);
    std::io::stdout().flush().unwrap();
    let password = read_password().unwrap();

    let new_entry = PasswordEntry {
        service: add_cmd.service.clone(),
        username: add_cmd.username.clone(),
        password,
    };

    entries.push(new_entry);

    if let Err(e) = write_vault(&master_password, &entries) {
        eprintln!("Error writing vault: {}", e);
    } else {
        println!("Password for '{}' added successfully.", add_cmd.service);
    }
}

fn handle_list_command() {
    let master_password = get_master_password();
    let entries = match read_vault(&master_password) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Error reading vault: {}", e);
            return;
        }
    };

    if entries.is_empty() {
        println!("No entries in vault.");
        return;
    }

    println!("Services:");
    for entry in entries {
        println!("- {}", entry.service);
    }
}

fn handle_get_command(get_cmd: GetCommand) {
    let master_password = get_master_password();
    let entries = match read_vault(&master_password) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Error reading vault: {}", e);
            return;
        }
    };

    let entry = entries.iter().find(|e| e.service == get_cmd.service);

    match entry {
        Some(e) => {
            let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
            ctx.set_contents(e.password.clone()).unwrap();
            println!("Password for '{}' copied to clipboard.", get_cmd.service);
        }
        None => {
            eprintln!("Service '{}' not found.", get_cmd.service);
        }
    }
}

fn handle_delete_command(delete_cmd: DeleteCommand) {
    let master_password = get_master_password();
    let mut entries = match read_vault(&master_password) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Error reading vault: {}", e);
            return;
        }
    };

    let initial_len = entries.len();
    entries.retain(|e| e.service != delete_cmd.service);

    if entries.len() == initial_len {
        eprintln!("Service '{}' not found.", delete_cmd.service);
        return;
    }

    if let Err(e) = write_vault(&master_password, &entries) {
        eprintln!("Error writing vault: {}", e);
    } else {
        println!(
            "Password for '{}' deleted successfully.",
            delete_cmd.service
        );
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add(add_cmd) => handle_add_command(add_cmd),
        Commands::Get(get_cmd) => handle_get_command(get_cmd),
        Commands::List => handle_list_command(),
        Commands::Delete(delete_cmd) => handle_delete_command(delete_cmd),
    }
}
