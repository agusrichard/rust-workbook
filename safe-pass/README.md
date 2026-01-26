# SafePass - Local Password Manager

SafePass is a command-line interface (CLI) tool designed to securely store and manage your passwords locally. It encrypts your sensitive login information (service, username, password) into a local file, accessible only via a master password.

## Requirements

- **Master Password:** A master password is required to unlock and manage the password vault.
- **Encrypted Storage:** All password entries are stored as ciphertext, not plain text, in a local file.
- **CLI Commands:** Interact with the vault using simple commands.
- **Clipboard Integration:** Automatically copies retrieved passwords to your clipboard for convenience.

## Features

- **`add`**: Add new service credentials (service name, username, and password).
- **`get`**: Retrieve the password for a specified service and automatically copy it to your clipboard.
- **`list`**: List all stored service names in your vault.
- **`delete`**: Remove a password entry for a specific service.

## Dependencies

SafePass relies on the following Rust crates:

- [`clap`](https://crates.io/crates/clap): For robust command-line argument parsing.
- [`magic-crypt`](https://crates.io/crates/magic-crypt): For encryption and decryption of the password vault.
- [`cli-clipboard`](https://crates.io/crates/cli-clipboard): For cross-platform clipboard access.
- [`rpassword`](https://crates.io/crates/rpassword): For securely reading passwords from the terminal (hidden input).
- [`serde`](https://crates.io/crates/serde) & [`serde_json`](https://crates.io/crates/serde_json): For serializing and deserializing password entries to/from JSON.
- [`dirs`](https://crates.io/crates/dirs): For determining the user's home directory to store the vault file.

## Installation

To build and run SafePass, you need to have Rust and Cargo installed.

1.  **Clone the repository:**

    ```bash
    git clone https://github.com/agusrichard/rust-workbook.git
    cd safe-pass
    ```

2.  **Build the project:**
    ```bash
    cargo build --release
    ```
    This will create an executable in the `target/release/` directory.

## Usage

You can run SafePass commands using `cargo run --` followed by the command and its arguments, or by directly executing the compiled binary (`./target/release/safe-pass`).

### Add a New Password Entry

```bash
cargo run -- add --service <service_name> --username <username>
# Example:
# cargo run -- add --service google --username my.email@gmail.com
```

You will be prompted to enter your master password and then the password for the new service.

### List All Stored Services

```bash
cargo run -- list
```

You will be prompted to enter your master password.

### Get a Password

```bash
cargo run -- get --service <service_name>
# Example:
# cargo run -- get --service google
```

You will be prompted for your master password. If the service is found, its password will be copied to your clipboard.

### Delete a Password Entry

```bash
cargo run -- delete --service <service_name>
# Example:
# cargo run -- delete --service google
```

You will be prompted for your master password.

## Vault File Location

The encrypted password vault is stored in your home directory as a hidden file named `.safepass.json.enc`.
For example:

- **Linux/macOS:** `~/.safepass.json.enc`
- **Windows:** `C:\Users\<YourUsername>\.safepass.json.enc`
