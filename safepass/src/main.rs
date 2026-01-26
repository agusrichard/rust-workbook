use clap::{Args, Parser, Subcommand};
use pam::Authenticator;
use std::process;
use std::io::{self, BufRead};

mod storage;
use storage::{Storage, FileStorage};

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
    /// The username for the service
    #[arg(short, long)]
    username: String,
}

#[derive(Args)]
struct DeleteCommand {
    /// The name of the service to delete
    #[arg(short, long)]
    service: String,
    /// The username for the service
    #[arg(short, long)]
    username: String,
}

#[derive(Args)]
struct UpdateCommand {
    /// The name of the service
    #[arg(short, long)]
    service: String,
    /// The username for the service
    #[arg(short, long)]
    username: String,
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
    /// Updates an existing password entry in the vault.
    ///
    /// You will be prompted for the master password and the new password.
    ///
    /// EXAMPLES:
    ///    safepass update --service google --username user@gmail.com
    Update(UpdateCommand),
    /// Retrieves a password from the vault.
    ///
    /// You will be prompted for the master password to decrypt the entry.
    ///
    /// EXAMPLES:
    ///    safepass get --service google --username user@gmail.com
    Get(GetCommand),
    /// Deletes a password entry from the vault.
    ///
    /// You will be prompted for the master password to confirm deletion.
    ///
    /// EXAMPLES:
    ///    safepass delete --service google --username user@gmail.com
    Delete(DeleteCommand),
    /// Lists all services and usernames stored in the vault.
    List,
}

trait PasswordInput {
    fn read_password(&self, prompt: &str) -> io::Result<String>;
}

struct StdPasswordInput;

impl PasswordInput for StdPasswordInput {
    fn read_password(&self, prompt: &str) -> io::Result<String> {
        if std::env::var("SAFEPASS_TEST_MODE").is_ok() {
            // Read from stdin in test mode
            let stdin = io::stdin();
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
            Ok(line.trim().to_string())
        } else {
            rpassword::prompt_password(prompt)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        }
    }
}

fn verify_user_password(input: &impl PasswordInput) -> String {
    if std::env::var("SAFEPASS_TEST_MODE").is_ok() {
        return "master_secret".to_string();
    }

    let username = whoami::username();

    const RETRIES: u32 = 3;
    for _ in 0..RETRIES {
        // Prompt for password securely (hidden input)
        let password =
            match input.read_password(&format!("Please enter password for {}: ", username)) {
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

fn handle_add_command(
    storage: &impl Storage,
    input: &impl PasswordInput,
    add_cmd: AddCommand,
    master_password: &str,
) -> Result<(), String> {
    match storage.load_entries() {
        Ok(entries) => {
            if entries
                .iter()
                .any(|e| e.service == add_cmd.service && e.username == add_cmd.username)
            {
                return Err(format!(
                    "Service '{}' with username '{}' already exists.",
                    add_cmd.service, add_cmd.username
                ));
            }
        }
        Err(e) => return Err(format!("Failed to load entries: {}", e)),
    }

    println!("Adding password for service: {}", add_cmd.service);
    let password = input.read_password(&format!(
        "Enter password for service `{}`: ",
        add_cmd.service
    )).map_err(|e| format!("Failed to read password: {}", e))?;

    let confirm = input.read_password("Confirm password: ")
        .map_err(|e| format!("Failed to read password: {}", e))?;

    if password != confirm {
        return Err("Passwords do not match!".to_string());
    }

    match storage.save_entry(
        &add_cmd.service,
        &add_cmd.username,
        &password,
        master_password,
    ) {
        Ok(_) => {
            println!("Password for {} added successfully.", add_cmd.service);
            Ok(())
        }
        Err(e) => Err(format!("Failed to save entry: {}", e)),
    }
}

fn handle_update_command(
    storage: &impl Storage,
    input: &impl PasswordInput,
    update_cmd: UpdateCommand,
    master_password: &str,
) -> Result<(), String> {
    match storage.load_entries() {
        Ok(entries) => {
            if !entries.iter().any(|e| {
                e.service == update_cmd.service && e.username == update_cmd.username
            }) {
                return Err(format!(
                    "Service '{}' with username '{}' not found.",
                    update_cmd.service, update_cmd.username
                ));
            }
        }
        Err(e) => return Err(format!("Failed to load entries: {}", e)),
    }

    println!("Updating password for service: {}", update_cmd.service);
    let password = input.read_password(&format!(
        "Enter new password for service `{}`: ",
        update_cmd.service
    )).map_err(|e| format!("Failed to read password: {}", e))?;

    let confirm = input.read_password("Confirm password: ")
        .map_err(|e| format!("Failed to read password: {}", e))?;

    if password != confirm {
        return Err("Passwords do not match!".to_string());
    }

    match storage.update_entry(
        &update_cmd.service,
        &update_cmd.username,
        &password,
        master_password,
    ) {
        Ok(_) => {
            println!("Password for {} updated successfully.", update_cmd.service);
            Ok(())
        }
        Err(e) => Err(format!("Failed to update entry: {}", e)),
    }
}

fn handle_get_command(
    storage: &impl Storage,
    get_cmd: GetCommand,
    master_password: &str,
) -> Result<(), String> {
    let entries = storage.load_entries().map_err(|e| format!("Failed to load entries: {}", e))?;

    let entry = entries
        .into_iter()
        .find(|e| e.service == get_cmd.service && e.username == get_cmd.username)
        .ok_or_else(|| format!(
            "Service '{}' with username '{}' not found.",
            get_cmd.service, get_cmd.username
        ))?;

    match entry.decrypt(master_password) {
        Ok(password) => {
            println!("Service: {}", entry.service);
            println!("Username: {}", entry.username);
            println!("Password: {}", password);
            Ok(())
        }
        Err(e) => Err(format!("Failed to decrypt password: {}", e)),
    }
}

fn handle_delete_command(
    storage: &impl Storage,
    delete_cmd: DeleteCommand
) -> Result<(), String> {
    match storage.delete_entry(&delete_cmd.service, &delete_cmd.username) {
        Ok(_) => {
            println!(
                "Service '{}' for user '{}' deleted successfully.",
                delete_cmd.service, delete_cmd.username
            );
            Ok(())
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(format!(
                    "Service '{}' with username '{}' not found.",
                    delete_cmd.service, delete_cmd.username
                ))
            } else {
                Err(format!("Failed to delete entry: {}", e))
            }
        }
    }
}

fn handle_list_command(storage: &impl Storage) -> Result<(), String> {
    let entries = storage.load_entries().map_err(|e| format!("Failed to load entries: {}", e))?;

    if entries.is_empty() {
        println!("No entries found in the vault.");
        return Ok(());
    }

    println!("{:<20} {:<30}", "SERVICE", "USERNAME");
    println!("{}", "-".repeat(50));
    for entry in entries {
        println!("{:<20} {:<30}", entry.service, entry.username);
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let storage = FileStorage::new();
    let input = StdPasswordInput;

    let master_password = verify_user_password(&input);

    let result = match cli.command {
        Commands::Add(add_cmd) => handle_add_command(&storage, &input, add_cmd, &master_password),
        Commands::Update(update_cmd) => handle_update_command(&storage, &input, update_cmd, &master_password),
        Commands::Get(get_cmd) => handle_get_command(&storage, get_cmd, &master_password),
        Commands::Delete(delete_cmd) => handle_delete_command(&storage, delete_cmd),
        Commands::List => handle_list_command(&storage),
    };

    if let Err(e) = result {
        eprintln!("{}", e);
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use storage::PasswordEntry;

    struct MockStorage {
        entries: RefCell<Vec<PasswordEntry>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                entries: RefCell::new(Vec::new()),
            }
        }
    }

    impl Storage for MockStorage {
        fn load_entries(&self) -> io::Result<Vec<PasswordEntry>> {
            Ok(self.entries.borrow().clone())
        }

        fn save_entry(
            &self,
            service: &str,
            username: &str,
            _password: &str,
            _master_password: &str,
        ) -> io::Result<()> {
            let mut entries = self.entries.borrow_mut();
            if entries.iter().any(|e| e.service == service && e.username == username) {
                return Err(io::Error::new(io::ErrorKind::AlreadyExists, "Exists"));
            }
            entries.push(PasswordEntry {
                service: service.to_string(),
                username: username.to_string(),
                encrypted_password: "enc".to_string(),
                salt: "salt".to_string(),
                nonce: "nonce".to_string(),
            });
            Ok(())
        }

        fn update_entry(
            &self,
            service: &str,
            username: &str,
            _new_password: &str,
            _master_password: &str,
        ) -> io::Result<()> {
            let mut entries = self.entries.borrow_mut();
            if let Some(pos) = entries.iter().position(|e| e.service == service && e.username == username) {
                entries[pos].encrypted_password = "updated".to_string();
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "Not found"))
            }
        }

        fn delete_entry(&self, service: &str, username: &str) -> io::Result<()> {
            let mut entries = self.entries.borrow_mut();
            let original_len = entries.len();
            entries.retain(|e| e.service != service || e.username != username);
            if entries.len() == original_len {
                Err(io::Error::new(io::ErrorKind::NotFound, "Not found"))
            } else {
                Ok(())
            }
        }
    }

    struct MockPasswordInput {
        inputs: RefCell<Vec<String>>,
    }

    impl MockPasswordInput {
        fn new(inputs: Vec<String>) -> Self {
            Self {
                inputs: RefCell::new(inputs),
            }
        }
    }

    impl PasswordInput for MockPasswordInput {
        fn read_password(&self, _prompt: &str) -> io::Result<String> {
            let mut inputs = self.inputs.borrow_mut();
            if inputs.is_empty() {
                Err(io::Error::new(io::ErrorKind::UnexpectedEof, "No more input"))
            } else {
                Ok(inputs.remove(0))
            }
        }
    }

    #[test]
    fn test_handle_add_command_success() {
        let storage = MockStorage::new();
        let input = MockPasswordInput::new(vec!["password".to_string(), "password".to_string()]);
        
        let cmd = AddCommand {
            service: "google".to_string(),
            username: "user".to_string(),
        };
        
        let result = handle_add_command(&storage, &input, cmd, "master");
        assert!(result.is_ok());
        
        assert_eq!(storage.entries.borrow().len(), 1);
        assert_eq!(storage.entries.borrow()[0].service, "google");
    }

    #[test]
    fn test_handle_add_command_mismatch() {
        let storage = MockStorage::new();
        let input = MockPasswordInput::new(vec!["pass1".to_string(), "pass2".to_string()]);
        
        let cmd = AddCommand {
            service: "yahoo".to_string(),
            username: "user".to_string(),
        };
        
        let result = handle_add_command(&storage, &input, cmd, "master");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Passwords do not match!");
        
        assert_eq!(storage.entries.borrow().len(), 0);
    }

    #[test]
    fn test_handle_add_command_duplicate() {
        let storage = MockStorage::new();
        // Pre-populate
        storage.save_entry("google", "user", "pass", "master").unwrap();
        
        let input = MockPasswordInput::new(vec![]); // Should not even ask for password
        
        let cmd = AddCommand {
            service: "google".to_string(),
            username: "user".to_string(),
        };
        
        let result = handle_add_command(&storage, &input, cmd, "master");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }
}
