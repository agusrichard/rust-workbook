# StockGuard Learning Project Plan

## How to use this plan

This is a learner-led plan, not an implementation recipe. Complete the tasks in order and do not move forward while a task's verification still fails. When you need help, ask about the smallest failing step and include the exact error, the relevant interface, and what you expected.

The plan deliberately introduces dependencies, directories, and abstractions **only when they are first needed**. Each task should leave the project running and verifiable. Avoid installing the whole future architecture at the beginning.

## Target outcome

At the end of the project:

- A public Axum server exposes availability, purchase, and SSE notification endpoints.
- A private Tonic server owns product and purchase data in SQLite.
- The public server uses unary and server-streaming gRPC to call the private server.
- SQLite transactions prevent overselling.
- Purchase notifications are emitted only after a successful commit.
- All internal gRPC traffic uses mutual TLS.
- Logs, error mapping, and documentation make the system easy to demonstrate.

## Working rules

- Keep each task in a small commit if you are using Git.
- Add a crate, feature, folder, or abstraction only when the current task uses it.
- Run formatting and compilation checks after every task.
- Keep the public HTTP models separate from generated Protobuf types.
- Prefer a direct design over extra abstraction.
- Never commit generated private keys or the SQLite database.
- Treat notifications as live-only; durable delivery is outside this project.
- If a task exceeds its timebox, record the blocker before expanding scope.

## Just-in-time dependency map

This is a preview, not a list to install immediately.

| First needed in | Dependency or artifact introduced |
|---|---|
| Task 1 | No new dependency; only the two Rust binaries |
| Task 2 | Tonic/Protobuf runtime, Protobuf build dependency, `proto/`, `build.rs`, shared library module |
| Task 3 | Tokio runtime, SQLx/SQLite, `migrations/`, `data/`, Inventory repository module |
| Task 4 | UUID and timestamp crate |
| Task 6 | Axum, Serde, JSON, and Tokio networking/time support |
| Task 7 | Tokio broadcast support and `tokio-stream` |
| Task 9 | `scripts/` and generated `certs/` |
| Task 10 | Tonic TLS feature |
| Task 11 | `tracing`, `tracing-subscriber`, and graceful-shutdown support |

## Suggested schedule

| Day | Tasks | Approximate focus time |
|---|---|---:|
| Completed | Task 0: baseline and tools | 20 minutes |
| Day 1 | Tasks 1–6: binaries, contract, database, transaction, unary gRPC, REST | 5–6 hours |
| Day 2 | Tasks 7–12: streaming, SSE, certificates, mTLS, resilience, final verification | 5–6 hours |

The timeboxes are guides. Understanding and verifying each boundary matters more than finishing within the estimate.

---

## Task 0 — Establish a clean baseline

**Status:** Completed

**Goal:** Know that the starting project and required local tools work before introducing application complexity.

### Checklist

- [x] Read `PROJECT_PROPOSAL.md` once from beginning to end.
- [x] Run the starter program and record its behavior.
- [x] Confirm the Rust toolchain and Cargo are available.
- [x] Confirm `protoc`, `sqlite3`, and `openssl` are available.
- [x] Decide to use Git commits as checkpoints.
- [x] Review ignore rules for build output, certificates, database files, and private keys.
- [x] Select REST port `8080`, gRPC port `50051`, and database path `data/stockguard.db`.

### Verified result

- [x] The starter program prints `Hello, world!`.
- [x] Rust `1.92.0` and Cargo `1.92.0` are available.
- [x] Protobuf compiler `29.4`, SQLite `3.43.2`, and OpenSSL `3.6.1` are available.
- [x] Git does not show a database, certificate, or private key ready to be committed.

---

## Task 1 — Establish two runnable binaries

**Goal:** Give each future server its own executable without adding libraries that are not yet used.

**Timebox:** 20 minutes

### Checklist

- [x] Create `src/bin/checkout_api.rs` with a minimal synchronous `main` function.
- [x] Create `src/bin/inventory_service.rs` with a minimal synchronous `main` function.
- [x] Make each binary print a distinct startup message.
- [x] Add no dependencies; this task only establishes Cargo binary targets.
- [x] Create no infrastructure directories yet.
- [x] Keep the original `src/main.rs` temporarily, or remove it if you understand how Cargo chooses binaries.
- [x] Run the formatter.
- [x] Inspect `cargo metadata` or Cargo's output and identify all available binary targets.

### Verify completion

- [x] `cargo check --all-targets` succeeds.
- [x] `cargo run --bin checkout_api` prints the Checkout API message and exits successfully.
- [x] `cargo run --bin inventory_service` prints the Inventory Service message and exits successfully.
- [x] You can explain why the project now has multiple binaries.
- [x] `Cargo.toml` still has no unused future dependencies.

**Learning checkpoint:** Explain the difference between a Cargo package, a library target, and a binary target.

---

## Task 2 — Define and generate the gRPC contract

**Goal:** Turn a small Protobuf contract into shared Rust client and server interfaces.

**Timebox:** 60 minutes

### Checklist

- [ ] Add only the Tonic/Protobuf crates needed to compile generated messages and services.
- [ ] Add `tonic-prost-build` under build dependencies rather than runtime dependencies.
- [ ] Create the `proto/` directory.
- [ ] Create `build.rs` for Protobuf generation.
- [ ] Create `src/lib.rs` or another shared module for generated types.
- [ ] Create `proto/inventory.proto` with package `inventory.v1`.
- [ ] Define unary `GetAvailability` and `CreatePurchase`.
- [ ] Define server-streaming `WatchPurchases`, even though its behavior will be implemented later.
- [ ] Give every field a stable number and use only fields required by the proposal.
- [ ] Configure `build.rs` to compile the `.proto` file.
- [ ] Include the generated module from the shared library target.
- [ ] Find the generated output under Cargo's build directory and inspect it.
- [ ] Keep Axum, SQLx, streaming adapters, TLS, and logging out of this task.

### Verify completion

- [ ] `cargo check --all-targets` invokes `protoc` and succeeds.
- [ ] The generated output contains an Inventory client and an Inventory server trait.
- [ ] The generated service has two unary methods and one server-streaming method.
- [ ] Both binaries can import the shared generated module.
- [ ] You can explain why generated files stay under `target/` rather than being edited manually.

**Learning checkpoint:** Explain how one `.proto` contract can produce both the caller interface and the server interface.

---

## Task 3 — Create and seed the SQLite database

**Goal:** Give the Inventory Service durable storage before adding business or network behavior.

**Timebox:** 60 minutes

### Checklist

- [ ] Add Tokio with only the runtime and macro features needed for async startup.
- [ ] Add SQLx with Tokio, SQLite, migration, and macro support.
- [ ] Create `migrations/` for versioned database schema files.
- [ ] Create `data/` for the ignored local database file.
- [ ] Create an Inventory repository/database module.
- [ ] Convert only the Inventory Service entry point to an async Tokio main function.
- [ ] Write the initial migration for `products` and `purchases`.
- [ ] Add primary keys, the product foreign key, and stock/quantity constraints.
- [ ] Create the SQLite pool during Inventory Service startup.
- [ ] Enable SQLite foreign-key enforcement on connections.
- [ ] Run embedded migrations during startup.
- [ ] Seed two or three products with idempotent inserts.
- [ ] Read the database path from an environment variable with `data/stockguard.db` as the local default.
- [ ] Use simple startup output or error propagation for now; structured logging comes later.
- [ ] Keep UUIDs, timestamp handling, Axum, streams, TLS, and tracing out of this task.

### Verify completion

- [ ] Starting the Inventory Service creates `data/stockguard.db`.
- [ ] The `sqlite3` CLI shows both tables and all seeded products.
- [ ] Restarting the service does not duplicate products or erase data.
- [ ] SQLite rejects negative stock and non-positive purchase quantities.
- [ ] `cargo check --all-targets` succeeds.
- [ ] The Checkout binary still runs without opening the database.

**Learning checkpoint:** Explain the difference between a migration, seed data, and ordinary application queries.

---

## Task 4 — Implement and test the purchase transaction

**Goal:** Make the central business rule correct entirely inside the database boundary.

**Timebox:** 75 minutes

### Checklist

- [ ] Add a UUID crate with random UUID generation enabled.
- [ ] Add one timestamp crate with UTC RFC 3339 formatting support.
- [ ] Add a repository operation for reading product availability.
- [ ] Define errors for unknown product, zero quantity, insufficient stock, and unexpected storage failure.
- [ ] Reject zero quantity before opening the transaction.
- [ ] Begin one SQLite transaction for the entire purchase.
- [ ] Use a conditional stock update so SQLite prevents stock from becoming negative.
- [ ] Distinguish unknown product from insufficient stock when no row is updated.
- [ ] Generate the purchase ID and UTC timestamp.
- [ ] Insert the purchase record inside the same transaction.
- [ ] Commit before returning success.
- [ ] Write repository tests for every expected outcome.
- [ ] Add a test where concurrent purchases compete for limited stock.
- [ ] Keep this behavior behind the repository boundary; do not expose it over gRPC yet.

### Verify completion

- [ ] A successful test decreases stock and creates exactly one purchase row.
- [ ] Each rejected purchase leaves both tables unchanged.
- [ ] Concurrent purchases never produce negative stock.
- [ ] Starting stock equals successful purchased quantity plus remaining stock.
- [ ] Returned IDs are valid UUIDs and timestamps are valid UTC RFC 3339 values.
- [ ] All repository tests pass repeatedly.

**Learning checkpoint:** Describe the race condition caused by reading, checking, and updating stock in separate transactions.

---

## Task 5 — Expose the unary Inventory gRPC API

**Goal:** Put the already-tested database behavior behind a network boundary.

**Timebox:** 60 minutes

### Checklist

- [ ] Create the Inventory gRPC service implementation module.
- [ ] Enable Tonic transport/server support if it is not already enabled.
- [ ] Convert domain results into Protobuf responses.
- [ ] Map domain errors to the agreed gRPC status codes.
- [ ] Implement `GetAvailability` using the repository.
- [ ] Implement `CreatePurchase` using the transaction operation.
- [ ] Keep status descriptions useful but free of SQL and internal paths.
- [ ] Start the Tonic server on port `50051` using temporary plaintext transport.
- [ ] Test through a schema-aware gRPC client or focused service-level tests.
- [ ] Leave `WatchPurchases` explicitly unimplemented until Task 7.
- [ ] Add no application-wide logging or TLS dependency yet.

### Verify completion

- [ ] A gRPC availability request returns seeded SQLite data.
- [ ] A successful gRPC purchase persists after the server restarts.
- [ ] Unknown product returns `NOT_FOUND`.
- [ ] Zero quantity returns `INVALID_ARGUMENT`.
- [ ] Insufficient stock returns `FAILED_PRECONDITION`.
- [ ] A failed request creates no purchase row.

**Temporary condition:** Plaintext gRPC is acceptable only until Task 10.

**Learning checkpoint:** Explain why database errors should become domain errors before becoming gRPC statuses.

---

## Task 6 — Build the public REST-to-gRPC path

**Goal:** Complete one vertical slice from public HTTP through gRPC to SQLite and back.

**Timebox:** 75 minutes

### Checklist

- [ ] Add Axum.
- [ ] Add Serde with derive support and JSON serialization.
- [ ] Enable the Tokio networking and time features needed by the HTTP server and deadlines.
- [ ] Create Checkout HTTP model, handler, and error-mapping modules.
- [ ] Convert the Checkout binary to an async server.
- [ ] Create its Tonic channel during startup and share the client through Axum state.
- [ ] Define public HTTP request/response structs separately from Protobuf types.
- [ ] Implement `GET /products/{product_id}/availability`.
- [ ] Implement `POST /purchases`.
- [ ] Validate obvious HTTP input before calling gRPC.
- [ ] Map expected gRPC statuses to the HTTP statuses in the proposal.
- [ ] Return one consistent, safe JSON error shape.
- [ ] Add a short deadline to unary gRPC requests.
- [ ] Use basic error output for now; structured tracing comes in Task 11.
- [ ] Keep SSE, TLS, and tracing out of this task.

### Verify completion

- [ ] An HTTP availability request returns Inventory-owned data.
- [ ] A successful HTTP purchase returns `201 Created` and changes SQLite.
- [ ] Unknown, zero-quantity, and insufficient-stock requests return 404, 400, and 409.
- [ ] Stopping the Inventory Service produces a controlled 503 or 504 instead of crashing Checkout.
- [ ] The Checkout API never opens SQLite directly.
- [ ] You can trace one request across JSON, Protobuf, SQLite, Protobuf, and JSON.

### Day 1 milestone

Stop here and demonstrate the complete unary path. Commit or record a checkpoint before adding streaming.

---

## Task 7 — Publish committed purchases over gRPC streaming

**Goal:** Implement the internal live event stream without involving the public SSE endpoint yet.

**Timebox:** 60 minutes

### Checklist

- [ ] Enable Tokio synchronization/broadcast support.
- [ ] Add `tokio-stream` with broadcast-wrapper support.
- [ ] Create an Inventory notification module.
- [ ] Add a broadcast channel owned by the Inventory Service.
- [ ] Make each `WatchPurchases` call subscribe to that channel.
- [ ] Adapt the receiver to the stream item type required by Tonic.
- [ ] Publish `PurchaseEvent` only after the SQLite commit succeeds.
- [ ] Handle closed and lagged receivers deliberately.
- [ ] Ensure a disconnected gRPC client releases its receiver.
- [ ] Keep the stream live-only; do not replay database history.

### Verify completion

- [ ] A connected gRPC subscriber receives one event for one successful purchase.
- [ ] A rejected or rolled-back purchase produces no event.
- [ ] Two successful purchases produce two events.
- [ ] Disconnecting the subscriber does not affect purchases or crash Inventory.
- [ ] A new subscriber receives future events but no historical events.

**Learning checkpoint:** Explain why an event sent before commit could lie to its consumers.

---

## Task 8 — Bridge gRPC events to public SSE

**Goal:** Let public clients observe the internal purchase stream using ordinary HTTP.

**Timebox:** 60 minutes

### Checklist

- [ ] Create a Checkout notification module.
- [ ] Add an Axum SSE route without a new crate unless the chosen approach genuinely requires one.
- [ ] Start a Checkout background task that calls `WatchPurchases`.
- [ ] Read events without blocking REST request handling.
- [ ] Forward received events into a Checkout-owned broadcast channel.
- [ ] Implement `GET /notifications/purchases` as SSE.
- [ ] Use a stable SSE event name such as `purchase.completed`.
- [ ] Serialize a public event model rather than the Protobuf type.
- [ ] Add an SSE keep-alive.
- [ ] Release each local receiver when its HTTP client disconnects.

### Verify completion

- [ ] An SSE client remains connected while normal REST requests still work.
- [ ] A successful purchase produces one matching SSE event.
- [ ] The event matches the purchase ID, product ID, quantity, and timestamp returned by REST.
- [ ] Multiple SSE clients receive the same new event.
- [ ] Closing one SSE client does not affect the others.
- [ ] A rejected purchase produces no SSE event.

**Learning checkpoint:** Explain the two fan-out boundaries: Inventory-to-gRPC and Checkout-to-SSE.

---

## Task 9 — Generate and inspect development certificates

**Goal:** Create the identities needed for mTLS before changing the servers' transport configuration.

**Timebox:** 45 minutes

### Checklist

- [ ] Create `scripts/` for the repeatable certificate-generation script.
- [ ] Generate certificate artifacts under the ignored `certs/` directory.
- [ ] Add no Rust dependency in this task.
- [ ] Create a repeatable OpenSSL script for one development CA.
- [ ] Generate a server certificate and key for the Inventory Service.
- [ ] Put the chosen Inventory service name in the server certificate SAN.
- [ ] Generate a distinct client certificate and key for Checkout.
- [ ] Choose filenames that clearly distinguish CA, server, and client material.
- [ ] Inspect issuer, subject, validity, key usage, and SAN.
- [ ] Confirm the generated directory and all keys remain ignored by Git.
- [ ] Document that these are local development credentials.

### Verify completion

- [ ] OpenSSL verifies both leaf certificates against the CA.
- [ ] The server certificate contains the exact domain Tonic will verify.
- [ ] The client and server use distinct identities and keys.
- [ ] Regenerating from an empty `certs/` directory succeeds.
- [ ] `git status` shows neither certificates nor private keys.

**Learning checkpoint:** Explain the separate jobs of a CA certificate, server identity, and client identity.

---

## Task 10 — Require mTLS for internal gRPC

**Goal:** Replace the temporary plaintext boundary with mutually authenticated transport.

**Timebox:** 75 minutes

### Checklist

- [ ] Enable Tonic's TLS feature only now.
- [ ] Add an Inventory server TLS configuration boundary.
- [ ] Add a Checkout client TLS configuration boundary.
- [ ] Configure Inventory with its server certificate and key.
- [ ] Configure Inventory to trust the development CA for client identities.
- [ ] Configure Checkout to trust the CA for the Inventory server.
- [ ] Configure Checkout with its client certificate and key.
- [ ] Set the expected domain to match the server certificate SAN.
- [ ] Use secured channels for both unary and streaming calls.
- [ ] Remove or disable plaintext gRPC.
- [ ] Return safe HTTP failures while keeping TLS detail internal.

### Verify completion

- [ ] Availability, purchases, and notifications work with trusted certificates.
- [ ] Omitting the Checkout client identity fails.
- [ ] A client certificate signed by another CA fails.
- [ ] Trusting the wrong server CA fails.
- [ ] Using the wrong server domain fails hostname verification.
- [ ] No option disables verification to make a test pass.

**Learning checkpoint:** State which side proves which identity during the mTLS handshake.

---

## Task 11 — Add resilience, structured tracing, and shutdown

**Goal:** Add operational behavior after the functional system is already understandable.

**Timebox:** 60 minutes

### Checklist

- [ ] Add `tracing`.
- [ ] Add `tracing-subscriber` with formatting and environment-filter support.
- [ ] Enable Tokio signal support for graceful shutdown.
- [ ] Add an HTTP tracing layer only if you choose to use one.
- [ ] Initialize a tracing subscriber in both binaries.
- [ ] Replace temporary prints with appropriately leveled structured events.
- [ ] Include purchase and product IDs where useful.
- [ ] Detect when Checkout's gRPC subscription ends or fails.
- [ ] Reconnect using a small bounded backoff rather than a tight loop.
- [ ] Keep the unary deadline separate from the long-lived stream lifetime.
- [ ] Log connection, disconnection, retry, lag, success, and rejection.
- [ ] Avoid logging keys, certificate contents, or raw internal errors to public clients.
- [ ] Add graceful shutdown for servers and background tasks.

### Verify completion

- [ ] Log verbosity changes when `RUST_LOG` changes.
- [ ] Stopping Inventory produces controlled stream-failure and retry logs without rapid spam.
- [ ] Restarting Inventory causes Checkout to reconnect.
- [ ] A purchase after reconnection reaches the SSE client.
- [ ] REST calls during downtime return controlled errors.
- [ ] Shutdown stops accepting new work and exits without a panic.
- [ ] Existing SQLite data survives every restart.

**Learning checkpoint:** Explain why unary calls need deadlines while a deliberate subscription should not have the same lifetime limit.

---

## Task 12 — Run the acceptance pass and document the project

**Goal:** Prove the system works from a clean setup and leave reproducible instructions.

**Timebox:** 45 minutes

### Checklist

- [ ] Add no production dependency unless verification exposes a genuine gap.
- [ ] Document required tools and the order of setup operations.
- [ ] Document how to generate certificates and create the database.
- [ ] Document how to start each server in a separate terminal.
- [ ] Add example availability, purchase, and SSE requests.
- [ ] State that notifications are live-only.
- [ ] Document ports and environment variables.
- [ ] Run formatting, compilation, and all tests.
- [ ] Start from regenerated certificates and a fresh database.
- [ ] Work through every acceptance criterion in `PROJECT_PROPOSAL.md`.
- [ ] Record one limitation and one possible next improvement.

### Verify completion

- [ ] A clean setup produces seeded products without manual database editing.
- [ ] Public clients need no knowledge of the gRPC schema.
- [ ] A purchase atomically updates SQLite and returns the expected response.
- [ ] A committed purchase appears once on an already-connected SSE client.
- [ ] Rejected purchases neither alter data nor emit events.
- [ ] All internal calls use mTLS and reject untrusted identities.
- [ ] Formatting, compilation, and tests pass.
- [ ] Following your documentation reproduces the demonstration.

### Final explanation exercise

Without looking at the code, explain:

1. Which server owns each public and internal API.
2. How Protobuf becomes Rust client and server interfaces.
3. How the SQLite transaction prevents overselling.
4. Why notifications are emitted only after commit.
5. How one event travels from gRPC streaming to SSE.
6. What each certificate proves during mTLS.
7. Which failures are safe to retry and which require idempotency.

If you can demonstrate the system and answer those questions, the learning project is complete.

## When asking for guidance

Use this format so help remains focused and you remain the implementer:

```text
Task:
What I am trying to achieve:
What I tried:
Observed result or exact error:
What I expected:
Relevant small code section:
```

Good requests include “help me understand this compiler error,” “review this transaction boundary,” or “show me the smallest example of adapting a broadcast receiver to a Tonic stream.” Solve and verify one task at a time rather than requesting the complete implementation.
