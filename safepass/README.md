# Safepass

Safepass is a secure, command-line password manager for macOS written in Rust. It protects your stored passwords using AES-256-GCM encryption, derived from your master password, and integrates with the system's Pluggable Authentication Modules (PAM) to verify your identity using your macOS user credentials (sudo password).

## Features

- **Secure Storage**: Uses AES-256-GCM for encryption and PBKDF2 for key derivation.
- **System Integration**: Authenticates using your local macOS user credentials via PAM.
- **Service & Username Support**: Stores multiple passwords for the same service under different usernames.
- **CLI Interface**: Simple and intuitive command-line interface.

## Prerequisites

- **macOS**: This tool is designed for macOS and relies on its PAM configuration.
- **Rust**: You need to have Rust installed to build the project. [Install Rust](https://www.rust-lang.org/tools/install).

## Installation

Clone the repository and build the project using Cargo:

```bash
git clone https://github.com/agusrichard/rust-workbook.git
cd safepass
cargo install --path .
```

This will install the `safepass` binary to your Cargo bin directory (usually `~/.cargo/bin`). Ensure this directory is in your `PATH`.

## Usage

### Add a Password

Add a new password for a service. You will be prompted to enter your master password (your macOS login password) and then the new password for the service.

```bash
safepass add --service <service-name> --username <username>
# Example
safepass add --service google --username user@example.com
```

### Get a Password

Retrieve and decrypt a password. You will be prompted for your master password.

```bash
safepass get --service <service-name> --username <username>
# Example
safepass get --service google --username user@example.com
```

### Update a Password

Update an existing password entry.

```bash
safepass update --service <service-name> --username <username>
# Example
safepass update --service google --username user@example.com
```

### List Entries

List all stored services and usernames. This command does not reveal passwords.

```bash
safepass list
```

### Delete a Password

Delete a password entry.

```bash
safepass delete --service <service-name> --username <username>
# Example
safepass delete --service google --username user@example.com
```

## Development

### Building

```bash
cargo build
```

### Testing

The project includes unit tests for internal logic and integration tests for the CLI.

```bash
cargo test
```

## Security

- **Encryption**: Passwords are encrypted with AES-256-GCM. A unique salt and nonce are generated for each entry.
- **Authentication**: Access to the vault requires authentication via PAM, ensuring only the logged-in user with valid credentials can decrypt the data.
- **Storage**: Data is stored in `~/.safepass/store.json`. While the passwords are encrypted, the service names and usernames are stored in plaintext to allow for listing and searching.

## License

[MIT License](LICENSE)
