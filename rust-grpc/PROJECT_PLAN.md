# StockGuard Learning Project Plan

## How to use this plan

This is a learner-led plan, not an implementation recipe. Complete the tasks in order and resist moving forward while a task's verification still fails. When you need help, ask about the smallest failing step and include the error message, the relevant interface, and what you expected to happen.

The plan intentionally describes responsibilities, decisions, and observable results without providing complete implementations. Small documentation examples and isolated API patterns may be useful while working, but the application code should remain yours.

## Target outcome

At the end of the project:

- A public Axum server exposes product availability, purchase, and SSE notification endpoints.
- A private Tonic server owns product and purchase data in SQLite.
- The public server calls the private server through unary and server-streaming gRPC.
- SQLite transactions prevent overselling.
- Purchase notifications are emitted only after a successful commit.
- All internal gRPC traffic uses mutual TLS.
- The project has enough logs, error mapping, and documentation to demonstrate each behavior.

## Working rules

- Keep each task on a separate small commit if you are using Git.
- Run formatting and compilation checks after every task.
- Add only the behavior required by the current task.
- Prefer a direct, understandable design over extra abstractions.
- Keep the public HTTP models separate from Protobuf-generated types.
- Never commit generated private keys or the SQLite database file.
- Treat notifications as live-only; durable delivery is outside this project.
- If a task exceeds its timebox, record the blocker and reduce optional work before expanding the design.

## Suggested schedule

| Day | Tasks | Approximate focus time |
|---|---|---:|
| Day 1 | Tasks 0–6: setup, contract, database, business rules, unary gRPC, REST | 5–6 hours |
| Day 2 | Tasks 7–12: gRPC streaming, SSE, certificates, mTLS, resilience, final verification | 5–6 hours |

The timeboxes are guides, not deadlines. The core learning goal is to understand each boundary before connecting the next one.

---

## Task 0 — Establish a clean baseline

**Goal:** Know that the starting project and required local tools work before introducing application complexity.

**Timebox:** 20 minutes

### Checklist

- [ ] Read `PROJECT_PROPOSAL.md` once from beginning to end.
- [ ] Run the current program and record its existing behavior.
- [ ] Confirm the Rust toolchain and Cargo are available.
- [ ] Confirm `protoc`, `sqlite3`, and `openssl` are available, or note which tools must be installed.
- [ ] Decide whether to use Git commits as checkpoints.
- [ ] Add or review ignore rules for `target/`, `certs/`, `data/`, `*.db`, and private-key files.
- [ ] Write down the ports and filenames you will use consistently, such as REST port `8080`, gRPC port `50051`, and `data/stockguard.db`.

### Verify completion

- [ ] `cargo run` succeeds with the starting application.
- [ ] You can report the installed versions of Rust, Protobuf compiler, SQLite, and OpenSSL.
- [ ] `git status` does not show a database, generated certificate, or private key ready to be committed.

**Learning checkpoint:** Explain why checking tools and ignored files now prevents confusing failures and accidental secret commits later.

---

## Task 1 — Create the project skeleton

**Goal:** Establish the two executable targets and module boundaries without implementing behavior.

**Timebox:** 30 minutes

### Checklist

- [ ] Add binary entry points for `checkout_api` and `inventory_service`.
- [ ] Add empty or minimal modules for the Checkout and Inventory responsibilities described in the proposal.
- [ ] Add the dependency groups needed for Tokio, Axum, Tonic, Protobuf, SQLx/SQLite, serialization, IDs, timestamps, streams, and tracing.
- [ ] Add the build dependency needed to generate Rust code from Protobuf.
- [ ] Create the `proto/`, `migrations/`, `scripts/`, `certs/`, and `data/` directories as appropriate.
- [ ] Keep both binaries minimal; do not connect the database or network yet.

### Verify completion

- [ ] `cargo check --all-targets` succeeds.
- [ ] `cargo run --bin checkout_api` starts the intended placeholder binary.
- [ ] `cargo run --bin inventory_service` starts the other placeholder binary.
- [ ] You can point to the module that will own each of these concerns: REST handlers, gRPC service, database access, and notifications.

**Learning checkpoint:** Explain why two binaries can share generated types without sharing database ownership.

---

## Task 2 — Define and generate the gRPC contract

**Goal:** Make Protobuf the explicit contract between the two servers.

**Timebox:** 45 minutes

### Checklist

- [ ] Create `proto/inventory.proto` with package `inventory.v1`.
- [ ] Define unary `GetAvailability`.
- [ ] Define unary `CreatePurchase`.
- [ ] Define server-streaming `WatchPurchases`.
- [ ] Define only the fields required by the proposal and give every field a stable number.
- [ ] Add `build.rs` and configure Protobuf code generation.
- [ ] Include the generated module from a shared library module.
- [ ] Locate the generated output under Cargo's build directory and inspect the generated client, server trait, and stream type.

### Verify completion

- [ ] `cargo check --all-targets` regenerates and compiles the Protobuf code.
- [ ] Changing a message field and rebuilding visibly changes the generated output.
- [ ] You can identify which generated trait the Inventory Service must implement.
- [ ] You can explain why `WatchPurchases` is server-streaming rather than client-streaming or bidirectional.

**Do not add yet:** database queries, REST handlers, TLS, or notification broadcasting.

---

## Task 3 — Create and seed the SQLite database

**Goal:** Give the Inventory Service durable storage with a reproducible schema.

**Timebox:** 60 minutes

### Checklist

- [ ] Write the first migration for `products` and `purchases`.
- [ ] Add primary keys, the product foreign key, and positive-stock/quantity constraints.
- [ ] Enable SQLite foreign-key enforcement when opening connections.
- [ ] Create a small SQLx SQLite pool during Inventory Service startup.
- [ ] Run embedded migrations at startup.
- [ ] Seed two or three recognizable products using an idempotent insert strategy.
- [ ] Keep the database path configurable through an environment variable with a sensible local default.
- [ ] Log migration and startup failures without exposing credentials or raw internals to public callers.

### Verify completion

- [ ] Starting the Inventory Service creates the database in the expected ignored location.
- [ ] SQLite inspection shows both tables, their constraints, and the seeded products.
- [ ] Restarting the service does not duplicate products or erase previous data.
- [ ] Attempting to insert negative stock or a non-positive purchase quantity directly in SQLite is rejected.
- [ ] `cargo check --all-targets` still succeeds.

**Learning checkpoint:** Explain the difference between a migration and seed data, and why foreign keys must be explicitly enabled in SQLite connections.

---

## Task 4 — Implement and test the purchase transaction

**Goal:** Make the core business operation correct before exposing it over the network.

**Timebox:** 75 minutes

### Checklist

- [ ] Add a repository operation for reading product availability.
- [ ] Add a purchase operation that rejects zero quantity.
- [ ] Begin one SQLite transaction for the entire purchase.
- [ ] Use a conditional stock update so the database itself prevents stock from going below zero.
- [ ] Distinguish an unknown product from insufficient stock when the conditional update affects no row.
- [ ] Insert the purchase record inside the same transaction.
- [ ] Commit before returning success.
- [ ] Define focused application errors that do not depend on HTTP or gRPC types.
- [ ] Write tests for success, unknown product, zero quantity, and insufficient stock.
- [ ] Add a concurrency test where two purchases compete for limited stock.

### Verify completion

- [ ] A successful test decreases stock and creates exactly one purchase row.
- [ ] Every rejected purchase leaves both tables unchanged.
- [ ] Competing purchases never produce negative stock.
- [ ] The sum of successful purchased quantities and remaining stock equals the starting stock.
- [ ] All repository/business tests pass repeatedly rather than only once.

**Learning checkpoint:** Describe what could go wrong if stock were read, checked, and updated in separate transactions.

---

## Task 5 — Expose the unary Inventory gRPC methods

**Goal:** Put the tested database behavior behind the internal gRPC contract.

**Timebox:** 60 minutes

### Checklist

- [ ] Implement the generated Inventory server trait.
- [ ] Connect `GetAvailability` to the repository read operation.
- [ ] Connect `CreatePurchase` to the transactional purchase operation.
- [ ] Generate the purchase ID and UTC timestamp in one clearly owned layer.
- [ ] Map domain errors to the agreed gRPC status codes.
- [ ] Keep error descriptions useful but free of SQL statements and internal paths.
- [ ] Start the Tonic server on the planned port without TLS for this temporary checkpoint.
- [ ] Add focused service tests or use a schema-aware gRPC client to exercise both methods.

### Verify completion

- [ ] A gRPC availability request returns seeded SQLite data.
- [ ] A successful gRPC purchase persists after the server restarts.
- [ ] Unknown products return `NOT_FOUND`.
- [ ] Zero quantity returns `INVALID_ARGUMENT`.
- [ ] Insufficient stock returns `FAILED_PRECONDITION`.
- [ ] A failed request does not create a purchase row.

**Temporary condition:** Plaintext gRPC is acceptable only until Task 10. Do not treat it as the finished security model.

---

## Task 6 — Build the public REST-to-gRPC path

**Goal:** Let an HTTP client use the Inventory Service without knowing about gRPC.

**Timebox:** 60 minutes

### Checklist

- [ ] Create the Checkout API's Tonic channel during startup and share a cloneable client through application state.
- [ ] Define public HTTP request and response structs instead of serializing Protobuf types directly.
- [ ] Implement `GET /products/{product_id}/availability`.
- [ ] Implement `POST /purchases`.
- [ ] Validate obvious HTTP input before making a gRPC call.
- [ ] Map gRPC statuses to the HTTP statuses listed in the proposal.
- [ ] Return a consistent, safe JSON error shape.
- [ ] Add a short deadline to each unary gRPC request.
- [ ] Add request-level tracing without logging sensitive material.

### Verify completion

- [ ] An HTTP availability request returns data owned by the Inventory Service.
- [ ] An HTTP purchase returns `201 Created` and changes SQLite data.
- [ ] Unknown, zero-quantity, and insufficient-stock requests return 404, 400, and 409 respectively.
- [ ] Stopping the Inventory Service makes the Checkout API return a controlled 503 or 504 rather than crash.
- [ ] The Checkout API never opens the SQLite database itself.

**Learning checkpoint:** Trace one purchase from JSON input through Protobuf, SQLite, Protobuf, and back to JSON.

### Day 1 milestone

Stop here and demonstrate the complete unary path. Commit or record a checkpoint before introducing streaming and TLS.

---

## Task 7 — Publish purchase events over server-streaming gRPC

**Goal:** Make the Inventory Service stream live events after committed purchases.

**Timebox:** 60 minutes

### Checklist

- [ ] Add an in-process Tokio broadcast channel owned by the Inventory Service.
- [ ] Subscribe to that channel for each `WatchPurchases` call.
- [ ] Adapt a receiver into the stream type required by Tonic.
- [ ] Publish a `PurchaseEvent` only after the SQLite commit succeeds.
- [ ] Handle closed and lagged broadcast receivers deliberately.
- [ ] Ensure dropping a gRPC client releases its subscription.
- [ ] Keep the stream live-only; do not query old purchases as replay.

### Verify completion

- [ ] Start a gRPC stream, make a successful purchase, and observe one matching event.
- [ ] A rejected purchase produces no event.
- [ ] Two successful purchases produce two events in commit order for a single service instance.
- [ ] Disconnecting the streaming client does not stop purchase processing or crash the server.
- [ ] A later subscriber receives future events but not historical events.

**Learning checkpoint:** Explain why the event must be sent after commit and what “live-only” means for a disconnected subscriber.

---

## Task 8 — Bridge gRPC events to public SSE

**Goal:** Make purchase notifications observable through the public Checkout API.

**Timebox:** 60 minutes

### Checklist

- [ ] Add a Checkout API background task that opens `WatchPurchases`.
- [ ] Read each gRPC event without blocking REST request handling.
- [ ] Forward events into a Checkout-owned local broadcast channel.
- [ ] Implement `GET /notifications/purchases` as an Axum SSE endpoint.
- [ ] Give the SSE event a stable name such as `purchase.completed`.
- [ ] Serialize a small public event model rather than forwarding the Protobuf type directly.
- [ ] Configure an SSE keep-alive.
- [ ] Clean up the local receiver when an HTTP client disconnects.

### Verify completion

- [ ] In terminal A, an SSE client remains connected to the notification endpoint.
- [ ] In terminal B, a successful purchase causes terminal A to receive one named event.
- [ ] The event contains the same purchase ID, product ID, quantity, and timestamp as the purchase response.
- [ ] Multiple SSE clients can observe the same new purchase.
- [ ] Closing one SSE client does not affect other clients or either server.
- [ ] A rejected purchase produces no SSE event.

**Learning checkpoint:** Explain the two fan-out boundaries: Inventory broadcast to gRPC streams, then Checkout broadcast to SSE clients.

---

## Task 9 — Generate and inspect development certificates

**Goal:** Understand the identities needed for mTLS before changing network configuration.

**Timebox:** 45 minutes

### Checklist

- [ ] Create a repeatable script that generates one development CA.
- [ ] Generate an Inventory Service certificate and private key signed by that CA.
- [ ] Include the chosen service name in the server certificate's Subject Alternative Name.
- [ ] Generate a Checkout API client certificate and private key signed by the CA.
- [ ] Choose clear filenames that distinguish CA, server, and client material.
- [ ] Inspect certificate issuer, subject, validity, key usage, and SAN with OpenSSL.
- [ ] Confirm all generated files remain ignored by Git.
- [ ] Document that the development CA and keys are not production credentials.

### Verify completion

- [ ] OpenSSL verifies both leaf certificates against the generated CA.
- [ ] The server certificate contains the exact domain name the Tonic client will verify.
- [ ] The client and server certificates have different identities and private keys.
- [ ] Running the script from a clean `certs/` directory recreates all required files.
- [ ] `git status` does not list private keys or generated certificates.

**Learning checkpoint:** Explain the distinct roles of a CA certificate, a server identity, and a client identity.

---

## Task 10 — Require mTLS for internal gRPC

**Goal:** Ensure the Inventory Service and Checkout API mutually authenticate.

**Timebox:** 75 minutes

### Checklist

- [ ] Enable the chosen Tonic TLS backend consistently.
- [ ] Configure the Inventory Service with its certificate and private key.
- [ ] Configure the Inventory Service to trust the development CA for client certificates.
- [ ] Configure the Checkout API channel to trust the development CA.
- [ ] Configure the Checkout API channel with its client certificate and private key.
- [ ] Configure the expected TLS domain name to match the server certificate SAN.
- [ ] Ensure both unary calls and the streaming subscription use secured channels.
- [ ] Remove or disable the plaintext internal endpoint.
- [ ] Keep TLS errors in internal logs while returning safe public failures.

### Verify completion

- [ ] Availability, purchases, and notifications work with the trusted certificates.
- [ ] Omitting the Checkout client identity causes connection failure.
- [ ] Using a client certificate signed by another CA causes connection failure.
- [ ] Trusting the wrong server CA causes connection failure.
- [ ] Using the wrong server domain causes hostname verification failure.
- [ ] No configuration option disables certificate verification to make a test pass.

**Learning checkpoint:** State which side proves which identity during the mTLS handshake.

---

## Task 11 — Add streaming resilience and useful observability

**Goal:** Make expected interruptions visible and recoverable without building a production platform.

**Timebox:** 45 minutes

### Checklist

- [ ] Detect when the Checkout API's gRPC stream ends or fails.
- [ ] Reconnect with a small bounded backoff instead of a tight retry loop.
- [ ] Avoid applying the unary two-second deadline to the lifetime of the long-lived stream.
- [ ] Log connection, disconnection, retry, lag, purchase success, and purchase rejection events.
- [ ] Include useful identifiers in logs, especially purchase and product IDs.
- [ ] Avoid logging certificate private keys, full database errors to public clients, or unnecessary request contents.
- [ ] Make shutdown behavior reasonable for both background tasks and servers.

### Verify completion

- [ ] Stop the Inventory Service while the Checkout API remains running.
- [ ] Logs show controlled stream failure and bounded reconnection attempts without rapid log spam.
- [ ] Restart the Inventory Service and observe the stream reconnect.
- [ ] A new purchase after reconnection reaches the SSE client.
- [ ] REST requests during downtime return controlled errors.
- [ ] Existing SQLite data remains intact after restart.

**Learning checkpoint:** Explain why a deadline is appropriate for unary RPCs but not for the intended lifetime of this subscription.

---

## Task 12 — Run the acceptance pass and document the project

**Goal:** Prove the full system works from a clean setup and leave instructions you can follow later.

**Timebox:** 45 minutes

### Checklist

- [ ] Add setup instructions for required tools, dependency build, database location, and certificate generation.
- [ ] Document the command for starting each server in a separate terminal.
- [ ] Document example availability, purchase, and SSE requests.
- [ ] State clearly that notifications are live-only.
- [ ] Document the chosen ports and environment variables.
- [ ] Run formatting, compilation checks, and all tests.
- [ ] Start from regenerated certificates and a fresh database.
- [ ] Work through every acceptance criterion in `PROJECT_PROPOSAL.md`.
- [ ] Record one known limitation and one possible next improvement.

### Verify completion

- [ ] A fresh setup produces seeded products without manual database editing.
- [ ] The public REST API works without public clients knowing the gRPC schema.
- [ ] A purchase atomically updates SQLite and returns the expected response.
- [ ] The committed purchase appears once on an already-connected SSE client.
- [ ] Rejected purchases neither alter data nor emit events.
- [ ] All internal calls use mTLS, and untrusted identities fail.
- [ ] Formatting, build checks, and tests all pass.
- [ ] You can delete generated local artifacts, follow your own instructions, and reproduce the demonstration.

### Final explanation exercise

Without looking at the code, explain:

1. Which server owns each public and internal API.
2. How the Protobuf contract becomes Rust client and server interfaces.
3. How the SQLite transaction prevents overselling.
4. Why notifications are emitted only after commit.
5. How one purchase travels from gRPC streaming to SSE.
6. What each certificate proves during mTLS.
7. Which failures are safe to retry and which may need idempotency.

If you can demonstrate the system and answer those questions, the learning project is complete.

## When asking for guidance

Use this short format so help stays focused and you remain the implementer:

```text
Task:
What I am trying to achieve:
What I tried:
Observed result or exact error:
What I expected:
Relevant small code section:
```

Good requests include “help me understand this compiler error,” “review this transaction boundary,” or “show me the smallest example of adapting a broadcast receiver to a Tonic stream.” Avoid requesting the complete project implementation; solve and verify one task at a time.
