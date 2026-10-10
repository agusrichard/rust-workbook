# StockGuard Learning Project Plan

## How to use this plan

This is a learner-led plan, not an implementation recipe. Complete one task at a time and do not move forward while its verification still fails. When asking for help, focus on the smallest failing step and include the exact error, the relevant interface, and what you expected.

The plan follows an incremental delivery style used in ordinary product work:

1. Establish the smallest required foundation.
2. Deliver one thin feature through every necessary layer.
3. Verify that feature from the outside.
4. Add the next feature only after the current increment works.

Do not add future RPC methods as placeholders. The Protobuf contract, database schema, HTTP API, and dependencies should grow only when the current feature requires them.

## Target outcome

At the end of the project:

- A public Axum server exposes product, purchase, and SSE notification endpoints.
- A private Tonic server owns product and purchase data in SQLite.
- The public server uses unary and server-streaming gRPC to call the private server.
- SQLite transactions prevent overselling.
- Purchase notifications are emitted only after a successful commit.
- All internal gRPC traffic uses mutual TLS.
- Logs, error mapping, and documentation make the system easy to demonstrate.

## Working rules

- Keep each completed task in a small commit when practical.
- Add a dependency, feature flag, folder, schema object, or API method only when the current task uses it.
- Keep every task buildable and independently verifiable.
- Run formatting and compilation checks after every task.
- Keep public HTTP models separate from generated Protobuf types.
- Prefer a direct implementation over abstractions needed only by imagined future work.
- Never commit generated private keys or the SQLite database.
- Treat notifications as live-only; durable delivery is outside this project.

## Naming convention

- Use **Inventory Service** for the private server that owns product stock and purchases.
- Use **Product** for the catalog resource in database, repository, gRPC, and HTTP code.
- Use `GetProduct`, `get_product`, and `GET /products/{product_id}` for the same read operation at each boundary.
- Use `stock` for the stored and returned product quantity.
- Use `product_id` at API boundaries and when one resource refers to a product; use `id` only for the product model's own primary-key field.
- Name transport-specific models by their role, such as `GetProductResponse` for Protobuf and `ProductResponse` for public JSON.

## Incremental delivery map

| Task | Working increment delivered |
|---|---|
| 0 | Verified local development baseline |
| 1 | Two runnable server binaries |
| 2 | SQLite product catalog with seed data |
| 3 | Product retrieval works from Checkout to Inventory over gRPC |
| 4 | Product retrieval is publicly accessible over REST |
| 5 | Purchases work correctly inside the database boundary |
| 6 | Purchases work end-to-end through REST and gRPC |
| 7 | Committed purchases produce an internal gRPC event stream |
| 8 | Purchase events are publicly observable through SSE |
| 9 | Development service identities and certificates exist |
| 10 | All internal gRPC traffic requires mTLS |
| 11 | The working system gains resilience, tracing, and graceful shutdown |
| 12 | A clean setup passes the complete acceptance demonstration |

## Suggested schedule

| Day | Tasks | Approximate focus time |
|---|---|---:|
| Completed | Tasks 0–1 | 40 minutes |
| Day 1 | Tasks 2–6: database, product retrieval, REST, purchase transaction, purchase API | 5–6 hours |
| Day 2 | Tasks 7–12: streaming, SSE, certificates, mTLS, resilience, acceptance | 5–6 hours |

The timeboxes are guides. Understanding and verifying each increment matters more than the estimate.

---

## Task 0 — Establish a clean baseline

**Status:** Completed

**Goal:** Confirm that the starting project and required local tools work.

### Checklist

- [x] Read `PROJECT_PROPOSAL.md`.
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

**Status:** Completed

**Goal:** Give each future server its own executable.

### Checklist

- [x] Create `src/bin/checkout_api.rs` with a minimal synchronous `main` function.
- [x] Create `src/bin/inventory_service.rs` with a minimal synchronous `main` function.
- [x] Make each binary print a distinct startup message.
- [x] Add no dependencies as part of this task.
- [x] Create no infrastructure directories as part of this task.
- [x] Decide how to handle the original `src/main.rs` binary.
- [x] Run the formatter.
- [x] Inspect Cargo's binary targets.

### Verified result

- [x] `cargo check --all-targets` succeeds at the Task 1 checkpoint.
- [x] `checkout_api` prints `Checkout API` and exits successfully.
- [x] `inventory_service` prints `Inventory Service` and exits successfully.
- [x] The project contains distinct executable targets for the two services.

**Learning checkpoint:** Explain the difference between a Cargo package, a library target, and a binary target.

---

## Task 2 — Establish the product database

**Goal:** Persist and inspect product stock before introducing any network API.

**Timebox:** 60 minutes

### Checklist

- [x] Add Tokio with only the runtime and macro features needed for async startup.
- [x] Add SQLx with Tokio, SQLite, migration, and macro support.
- [x] Create `migrations/` and `data/` only now that they are needed.
- [x] Add a first migration that creates only the `products` table.
- [x] Give products an ID, name, and non-negative stock constraint.
- [x] Create a small Inventory database/repository module.
- [x] Convert only `inventory_service` to an async Tokio entry point.
- [x] Read the database location from an environment variable with `data/stockguard.db` as its local default.
- [x] Open a small SQLite connection pool and enable foreign-key enforcement.
- [x] Run embedded migrations at Inventory startup.
- [x] Seed two or three products using idempotent inserts.
- [x] Add a repository operation that reads one product by ID.
- [x] Keep Protobuf, gRPC behavior, purchases, Axum, streaming, TLS, and tracing out of this increment.

### Verify completion

- [x] Starting Inventory creates `data/stockguard.db`.
- [x] The `sqlite3` CLI shows the `products` table and seeded rows.
- [x] Restarting Inventory neither erases nor duplicates products.
- [x] SQLite rejects negative stock.
- [x] The repository returns a known product and reports an unknown product predictably.
- [x] Checkout still runs without opening SQLite.
- [x] Formatting, `cargo check --all-targets`, and database-focused tests pass.

**Learning checkpoint:** Explain the difference between a migration, seed data, and an ordinary repository query.

---

## Task 3 — Deliver product retrieval over gRPC

**Goal:** Complete the first internal feature from Checkout to Inventory using one unary RPC.

**Timebox:** 90 minutes

### Checklist

- [x] Add Tonic, Prost, and the Tonic Prost runtime required by generated code.
- [x] Add `tonic-prost-build` under build dependencies.
- [x] Create or finish `proto/inventory.proto` with package `inventory.v1`.
- [x] Define only the `GetProduct` RPC and its request and response messages.
- [x] Do not define `CreatePurchase` or `WatchPurchases` yet.
- [x] Create or finish root-level `build.rs` and compile the `.proto` file.
- [x] Create or finish `src/lib.rs` as the shared generated-contract module.
- [x] Inspect the generated client, server trait, request, and response types.
- [x] Implement `GetProduct` by calling the Task 2 repository operation.
- [x] Map an unknown product to gRPC `NOT_FOUND`.
- [x] Start Inventory's Tonic server on port `50051` using temporary plaintext transport.
- [x] Convert Checkout to an async entry point.
- [x] Make Checkout create a Tonic client, request one product, print the result, and exit.
- [x] Allow the smoke-test product ID to be changed without recompiling, using a simple argument or environment value.
- [x] Keep Axum, purchase behavior, streaming, TLS, and tracing out of this increment.

### Verify completion

- [x] `cargo check --all-targets` invokes `protoc` and succeeds.
- [x] Inventory remains running on port `50051`.
- [x] Running Checkout for a seeded ID prints the product and its stock.
- [x] Running Checkout for an unknown ID reports `NOT_FOUND` without crashing either process.
- [x] The response is sourced from SQLite rather than hard-coded in the gRPC service.
- [x] The generated Inventory service contains exactly one RPC.
- [x] Formatting and focused tests pass.

**Learning checkpoint:** Trace the product request through the generated client, network transport, generated server, repository, and back.

---

## Task 4 — Expose product retrieval through public REST

**Goal:** Turn the internal product lookup into the first public API endpoint.

**Timebox:** 60 minutes

### Checklist

- [ ] Add Axum and Serde with only the features required for HTTP and JSON.
- [ ] Enable the Tokio networking and time features needed by the HTTP server and RPC deadline.
- [ ] Define a public `ProductResponse` type separate from the Protobuf response.
- [ ] Change Checkout from a one-shot smoke client into a long-running Axum server on port `8080`.
- [ ] Create the Tonic channel during Checkout startup and share the client through application state.
- [ ] Implement only `GET /products/{product_id}`.
- [ ] Add a short deadline to the internal unary call.
- [ ] Map gRPC `NOT_FOUND` to HTTP 404.
- [ ] Map Inventory unavailability or deadline failure to a controlled 503 or 504.
- [ ] Return a consistent JSON error shape.
- [ ] Remove the temporary startup product request once the HTTP handler replaces it.
- [ ] Keep purchases, SSE, TLS, and structured tracing out of this increment.

### Verify completion

- [ ] A public request for a seeded product returns HTTP 200 with SQLite-backed JSON.
- [ ] An unknown product returns HTTP 404 with safe JSON.
- [ ] Stopping Inventory causes a controlled 503 or 504 rather than a Checkout crash.
- [ ] Restarting Inventory allows subsequent product requests to succeed.
- [ ] Checkout never opens SQLite directly.
- [ ] Formatting, compilation, and focused tests pass.

**Learning checkpoint:** Explain why the public JSON model should not be the generated Protobuf model.

---

## Task 5 — Implement purchase persistence and business rules

**Goal:** Make purchasing correct inside the database boundary before exposing another API.

**Timebox:** 90 minutes

### Checklist

- [ ] Add a second migration that creates the `purchases` table without changing the first migration.
- [ ] Add the product foreign key and a positive-quantity constraint.
- [ ] Add a UUID crate with random UUID generation enabled.
- [ ] Add one timestamp crate with UTC RFC 3339 formatting support.
- [ ] Define domain results and errors without depending on HTTP or gRPC types.
- [ ] Reject zero quantity before opening a transaction.
- [ ] Start one SQLite transaction for the purchase.
- [ ] Decrease stock with a conditional update that requires sufficient stock.
- [ ] Distinguish an unknown product from insufficient stock when no row is updated.
- [ ] Insert the purchase ID, product ID, quantity, and timestamp within the same transaction.
- [ ] Commit before returning success.
- [ ] Add tests for success, unknown product, zero quantity, and insufficient stock.
- [ ] Add a test where concurrent purchases compete for limited stock.
- [ ] Keep the Protobuf and HTTP contracts unchanged in this increment.

### Verify completion

- [ ] Migrating an existing Task 4 database preserves its products and adds `purchases`.
- [ ] A successful repository call decreases stock and inserts exactly one purchase.
- [ ] Every rejected purchase leaves both tables unchanged.
- [ ] Concurrent attempts never make stock negative.
- [ ] Starting stock equals successful purchased quantity plus remaining stock.
- [ ] Returned purchase IDs and timestamps have the expected formats.
- [ ] All database and business tests pass repeatedly.

**Learning checkpoint:** Describe the race condition caused by checking and updating stock in separate transactions.

---

## Task 6 — Deliver purchases through gRPC and REST

**Goal:** Extend the working application with an end-to-end purchase feature.

**Timebox:** 75 minutes

### Checklist

- [ ] Extend the Protobuf service with `CreatePurchase` only now.
- [ ] Add only the request and response fields needed by the purchase feature.
- [ ] Rebuild and inspect how the generated client and server interfaces changed.
- [ ] Implement `CreatePurchase` by calling the tested Task 5 transaction.
- [ ] Map unknown product, zero quantity, and insufficient stock to appropriate gRPC statuses.
- [ ] Define separate public HTTP purchase request and response models.
- [ ] Implement `POST /purchases` in Checkout.
- [ ] Validate obvious public input before making the internal call.
- [ ] Map expected gRPC statuses to HTTP 404, 400, and 409.
- [ ] Apply the existing unary deadline policy.
- [ ] Keep streaming, SSE, TLS, and structured tracing out of this increment.

### Verify completion

- [ ] A valid HTTP purchase returns 201 and a purchase ID.
- [ ] SQLite shows both the new purchase and reduced product stock.
- [ ] The product endpoint reports the updated stock.
- [ ] Unknown, zero-quantity, and insufficient-stock requests return 404, 400, and 409.
- [ ] Rejected HTTP requests make no database changes.
- [ ] Two competing HTTP purchases cannot oversell the product.
- [ ] The generated Inventory service now contains exactly two unary RPCs.
- [ ] Formatting, compilation, and feature tests pass.

### Day 1 milestone

Demonstrate product retrieval and purchasing through the public REST API. Commit or record a checkpoint before introducing streaming.

---

## Task 7 — Deliver committed purchase events over gRPC streaming

**Goal:** Add the internal notification feature only after purchases work end-to-end.

**Timebox:** 75 minutes

### Checklist

- [ ] Extend the Protobuf service with `WatchPurchases` only now.
- [ ] Add `WatchPurchasesRequest` and `PurchaseEvent` messages.
- [ ] Confirm the generated method is server-streaming.
- [ ] Enable Tokio broadcast support.
- [ ] Add `tokio-stream` with the feature needed to wrap a broadcast receiver.
- [ ] Create a broadcast channel owned by Inventory.
- [ ] Make each `WatchPurchases` call subscribe to that channel.
- [ ] Adapt the receiver to Tonic's expected stream item type.
- [ ] Publish an event only after the purchase transaction commits.
- [ ] Handle closed and lagged receivers deliberately.
- [ ] Start one Checkout background task that subscribes to the gRPC stream.
- [ ] Temporarily print received events so this internal increment is observable.
- [ ] Keep the public HTTP API unchanged in this increment.

### Verify completion

- [ ] Checkout establishes the stream while continuing to serve REST requests.
- [ ] One successful purchase produces one printed event with matching data.
- [ ] A rejected or rolled-back purchase produces no event.
- [ ] Two successful purchases produce two events.
- [ ] Disconnecting Checkout does not stop purchase processing or crash Inventory.
- [ ] A later subscriber receives future events but no historical events.
- [ ] The generated Inventory service now contains two unary RPCs and one server-streaming RPC.

**Learning checkpoint:** Explain why publishing an event before the database commit could mislead consumers.

---

## Task 8 — Expose purchase notifications through SSE

**Goal:** Replace the temporary event output with a useful public notification stream.

**Timebox:** 60 minutes

### Checklist

- [ ] Add direct JSON serialization support only if the SSE implementation needs it.
- [ ] Define a public purchase-event type separate from the Protobuf event.
- [ ] Add a Checkout-owned broadcast channel for connected HTTP clients.
- [ ] Forward events from the existing gRPC subscriber into the local channel instead of printing them.
- [ ] Implement `GET /notifications/purchases` using Axum SSE.
- [ ] Use a stable event name such as `purchase.completed`.
- [ ] Add an SSE keep-alive.
- [ ] Release each receiver when its HTTP client disconnects.
- [ ] Keep notifications live-only; do not add database replay or a message broker.

### Verify completion

- [ ] An SSE client remains connected while ordinary REST requests still work.
- [ ] A successful purchase produces one matching SSE event.
- [ ] The event matches the purchase response's ID, product, quantity, and timestamp.
- [ ] Multiple SSE clients receive the same new event.
- [ ] Closing one SSE client does not affect the others.
- [ ] A rejected purchase produces no SSE event.
- [ ] A newly connected client does not receive historical purchases.

**Learning checkpoint:** Explain the Inventory-to-gRPC and Checkout-to-SSE fan-out boundaries.

---

## Task 9 — Generate development service identities

**Goal:** Create and understand the certificates needed before changing transport security.

**Timebox:** 45 minutes

### Checklist

- [ ] Create `scripts/` only now that certificate automation is needed.
- [ ] Write a repeatable OpenSSL script for one development CA.
- [ ] Generate artifacts under the ignored `certs/` directory.
- [ ] Generate an Inventory server certificate and private key signed by the CA.
- [ ] Put the chosen Inventory service name in the server certificate SAN.
- [ ] Generate a distinct Checkout client certificate and key signed by the CA.
- [ ] Inspect issuer, subject, validity, key usage, and SAN.
- [ ] Confirm generated certificates and keys remain ignored by Git.
- [ ] Document that these identities are local development credentials.
- [ ] Add no Rust dependency or TLS configuration in this increment.

### Verify completion

- [ ] OpenSSL verifies both leaf certificates against the CA.
- [ ] The server certificate contains the exact domain Tonic will verify.
- [ ] Client and server certificates use distinct identities and private keys.
- [ ] Running the script from a clean `certs/` directory recreates every artifact.
- [ ] `git status` lists no generated certificate or private key.

**Learning checkpoint:** Explain the separate roles of a CA certificate, server identity, and client identity.

---

## Task 10 — Require mTLS for every internal call

**Goal:** Secure the already-working unary and streaming gRPC traffic.

**Timebox:** 75 minutes

### Checklist

- [ ] Enable the chosen Tonic TLS backend only now.
- [ ] Configure Inventory with its server certificate and private key.
- [ ] Configure Inventory to trust the development CA for client identities.
- [ ] Configure Checkout to trust the CA for the Inventory server.
- [ ] Configure Checkout with its client certificate and private key.
- [ ] Set the expected server domain to match the certificate SAN.
- [ ] Reuse the secure channel for product retrieval, purchases, and the event subscription.
- [ ] Remove or disable plaintext gRPC.
- [ ] Keep detailed TLS errors internal while returning safe public failures.

### Verify completion

- [ ] Product retrieval, purchasing, and SSE notifications work with trusted certificates.
- [ ] Omitting the Checkout client identity fails.
- [ ] A client certificate signed by another CA fails.
- [ ] Trusting the wrong server CA fails.
- [ ] Using the wrong server domain fails hostname verification.
- [ ] No configuration disables certificate verification to make a test pass.

**Learning checkpoint:** State which identity each side proves during the mTLS handshake.

---

## Task 11 — Add resilience, structured tracing, and shutdown

**Goal:** Add operational behavior after all core features work.

**Timebox:** 60 minutes

### Checklist

- [ ] Add `tracing`.
- [ ] Add `tracing-subscriber` with formatting and environment-filter support.
- [ ] Enable Tokio signal support for graceful shutdown.
- [ ] Initialize a tracing subscriber in both binaries.
- [ ] Replace temporary prints with appropriately leveled structured events.
- [ ] Include purchase and product IDs where useful.
- [ ] Detect when Checkout's gRPC subscription ends or fails.
- [ ] Reconnect using a small bounded backoff rather than a tight loop.
- [ ] Keep unary deadlines separate from the long-lived stream lifetime.
- [ ] Log connection, disconnection, retry, lag, success, and rejection.
- [ ] Avoid logging private keys, certificate contents, or unsafe public error details.
- [ ] Add graceful shutdown for both servers and Checkout's background task.

### Verify completion

- [ ] Log verbosity changes when `RUST_LOG` changes.
- [ ] Stopping Inventory produces controlled failure and retry logs without rapid spam.
- [ ] Restarting Inventory causes Checkout to reconnect.
- [ ] A purchase after reconnection reaches the SSE client.
- [ ] REST calls during downtime return controlled errors.
- [ ] Shutdown stops accepting work and exits without panic.
- [ ] SQLite data survives every restart.

**Learning checkpoint:** Explain why unary calls need deadlines while a deliberate subscription should not share the same lifetime limit.

---

## Task 12 — Run the acceptance pass and document the project

**Goal:** Prove the system works from a clean setup and leave reproducible instructions.

**Timebox:** 45 minutes

### Checklist

- [ ] Add no production dependency unless verification exposes a genuine gap.
- [ ] Document required tools and setup order.
- [ ] Document database creation and certificate generation.
- [ ] Document how to start each server in a separate terminal.
- [ ] Add example product, purchase, and SSE requests.
- [ ] State that notifications are live-only.
- [ ] Document ports and environment variables.
- [ ] Run formatting, compilation, and all tests.
- [ ] Start from regenerated certificates and a fresh database.
- [ ] Work through every acceptance criterion in `PROJECT_PROPOSAL.md`.
- [ ] Record one limitation and one possible next improvement.

### Verify completion

- [ ] A clean setup creates and seeds the product database without manual editing.
- [ ] Public clients require no knowledge of the gRPC schema.
- [ ] A purchase atomically updates SQLite and returns the expected response.
- [ ] A committed purchase appears once on an already-connected SSE client.
- [ ] Rejected purchases neither alter data nor emit events.
- [ ] All internal calls use mTLS and reject untrusted identities.
- [ ] Formatting, compilation, and tests pass.
- [ ] Following the documentation reproduces the complete demonstration.

### Final explanation exercise

Without looking at the code, explain:

1. Which server owns each public and internal API.
2. How the Protobuf contract becomes Rust client and server interfaces.
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
