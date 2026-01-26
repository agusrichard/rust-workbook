# Safepass

Safepass is a command-line tool designed to securely store and retrieve passwords, protected by your macOS user credentials. It leverages system authentication (PAM) to verify identity before allowing access to the password vault.

## Project Overview

This project is written in Rust and provides a CLI interface for managing passwords.

**Key Features:**
*   **Secure Storage:** Passwords are encrypted using AES-256-GCM. Keys are derived from the user's master password using PBKDF2.
*   **System Authentication:** Integrates with PAM (Pluggable Authentication Modules) to authenticate the user using their system login credentials (e.g., sudo password).
*   **CLI Interface:** Built with `clap` for a robust command-line experience.

**Architecture:**
*   **`src/main.rs`**: The entry point of the application. It handles CLI argument parsing, user interaction (prompts), and orchestrates the command logic. It defines the `PasswordInput` trait for abstracting user input.
*   **`src/storage.rs`**: Handles the persistence layer. It defines the `Storage` trait and `FileStorage` implementation. It manages reading/writing the JSON store and handles encryption/decryption of password entries.

## Building and Running

The project is managed with `cargo`.

**Build:**
```bash
cargo build
```

**Run:**
```bash
cargo run -- <command> [args]
# Example:
cargo run -- add --service google --username myuser
```

**Testing:**
The project includes both unit tests and integration tests.
```bash
cargo test
```
*   **Unit Tests:** Located within `src/main.rs` and `src/storage.rs` to test internal logic and mocking.
*   **Integration Tests:** Located in `tests/cli.rs`, verifying the CLI behavior end-to-end using `assert_cmd`.

## Development Conventions

*   **Traits for Testability:** The project uses dependency injection via traits (`Storage`, `PasswordInput`) to make the CLI handlers unit-testable without relying on actual file I/O or user input.
*   **Error Handling:** Command handlers return `Result<(), String>` (or similar) to allow for clean error reporting and testing of failure states.
*   **Security:** Sensitive data (passwords) is handled carefully. `rpassword` is used for hidden input. `aes-gcm` and `pbkdf2` are used for cryptography.

## Usage

The tool stores its data in `~/.safepass/store.json`.

**Commands:**
*   `add`: Add a new password. Requires service name and username.
*   `get`: Retrieve a password. Requires service name and username.
*   `update`: Update an existing password.
*   `delete`: Delete a password entry.
*   `list`: List all stored services and usernames.

**Environment Variables (for testing):**
*   `SAFEPASS_TEST_MODE`: If set, bypasses PAM authentication and reads passwords from stdin instead of TTY.
*   `SAFEPASS_STORE_DIR`: Overrides the default storage directory path.
