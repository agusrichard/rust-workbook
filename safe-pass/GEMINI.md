# Gemini Codelab Context: SafePass Project

This document provides context for the SafePass project, a command-line password manager written in Rust.

## Project Overview

SafePass is a CLI tool for securely storing and managing passwords in a local, encrypted file. It uses a master password to encrypt and decrypt the password vault.

### Main Technologies & Crates

-   **Language:** Rust (2021 Edition)
-   **CLI Framework:** `clap` (derive feature) is used for parsing command-line arguments and subcommands.
-   **Encryption:** `magic-crypt` is used for AES-256 encryption of the vault file.
-   **Data Serialization:** `serde` and `serde_json` are used to serialize and deserialize the password entries into a JSON format before encryption.
-   **Clipboard:** `cli-clipboard` provides cross-platform access to the system clipboard for copying passwords.
-   **Secure Input:** `rpassword` is used to read the master password from the terminal without echoing it to the screen.
-   **Directory Management:** `dirs` is used to locate the user's home directory for storing the vault file.

### Architecture

-   The main executable is in `src/main.rs`.
-   The application is structured around a `Cli` struct and a `Commands` enum, both defined using `clap`.
-   The core logic is organized into handler functions for each command (`handle_add_command`, `handle_get_command`, etc.).
-   Password data is stored in a `PasswordEntry` struct, and a `Vec<PasswordEntry>` is serialized to JSON and then encrypted.
-   The encrypted vault is stored in a hidden file named `.safepass.json.enc` in the user's home directory.

## Building and Running

### Build

To build the project in release mode:

```bash
cargo build --release
```

### Run

To run the application using Cargo:

```bash
cargo run -- <COMMAND>
```

**Commands:**

-   **`add`**: Adds a new password entry.
    ```bash
    cargo run -- add --service <service_name> --username <username>
    ```
-   **`get`**: Gets a password for a service and copies it to the clipboard.
    ```bash
    cargo run -- get --service <service_name>
    ```
-   **`list`**: Lists all stored services.
    ```bash
    cargo run -- list
    ```
-   **`delete`**: Deletes a password entry.
    ```bash
    cargo run -- delete --service <service_name>
    ```
-   **`help`**: Shows the help message.
    ```bash
    cargo run -- --help
    ```

### Testing

There are no automated tests in this project yet.

## Development Conventions

-   The code is formatted using standard `rustfmt`.
-   Error handling is done primarily through `Result` and `Box<dyn std::error::Error>`, with messages printed to `stderr`.
-   The `main` function acts as a dispatcher, parsing commands and calling the appropriate handler function.
-   Helper functions like `get_vault_path`, `read_vault`, and `write_vault` encapsulate specific logic.
-   Detailed help messages are generated using `clap`'s `verbatim_doc_comment` feature. Doc comments on the `Commands` enum and the argument structs (`AddCommand`, etc.) are used to provide descriptions and examples.
