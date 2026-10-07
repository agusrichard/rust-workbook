# Project Proposal: StockGuard

## Summary

Build a small **e-commerce purchase and stock notification system** with two Rust servers:

1. **Checkout API** — exposes a public JSON/REST API and a public notification stream using Axum.
2. **Inventory Service** — stores products and purchases in SQLite and exposes a private gRPC API using Tonic.

When a customer purchases an item, the Checkout API calls the Inventory Service over **mutual TLS (mTLS)**. The Inventory Service atomically updates stock, records the purchase in SQLite, and publishes a purchase event through a **server-streaming gRPC method**. The Checkout API forwards those events to public clients using Server-Sent Events (SSE).

The project is deliberately limited to roughly **10–12 hours across two leisure days**. It provides hands-on practice with REST, unary and streaming gRPC, Protobuf, SQLite transactions, error mapping, and transport security without requiring a frontend or external infrastructure.

## Why this is a good real-world case

An online shop should not let public web or mobile clients access its inventory system directly. Instead:

- The Checkout API owns the public HTTP contract.
- The Inventory Service owns stock and purchase consistency.
- SQLite provides simple, durable local storage.
- Unary gRPC handles availability checks and purchases.
- Server-streaming gRPC carries live purchase events to another service.
- mTLS encrypts internal traffic and lets both services verify the other's identity.
- SSE gives browsers and command-line clients a simple way to observe notifications.

The same pattern appears in ticket booking, hotel reservations, warehouse systems, and operational dashboards.

## User stories

> As a shopper, I can check a product's availability and purchase a quantity through a public REST API.

> As an operator, I can subscribe to a live HTTP event stream and receive a notification after each purchase is successfully committed.

## Architecture

```text
                     public REST/JSON
Customer  ------------------------------------>  Checkout API
          GET /products/{id}/availability       (Axum, port 8080)
          POST /purchases                               |
                                                       | unary gRPC over mTLS
                                                       v
                                                Inventory Service
                                                (Tonic, port 50051)
                                                       |
                                                       | SQLite transaction
                                                       v
                                                stockguard.db

                     public SSE                       ^
Operator  <------------------------------------ Checkout API
          GET /notifications/purchases                ^
                                                       |
                                                       | server-streaming gRPC
                                                       | over mTLS
                                                Inventory Service
```

The Checkout API is an HTTP server and a gRPC client. It keeps one long-lived subscription to the Inventory Service's purchase-event stream, then fans received events out to connected SSE clients.

## Minimum viable scope

### Public REST API

#### `GET /products/{product_id}/availability`

Successful response:

```json
{
  "product_id": "keyboard-01",
  "name": "Mechanical Keyboard",
  "available_quantity": 10
}
```

#### `POST /purchases`

Request:

```json
{
  "product_id": "keyboard-01",
  "quantity": 2
}
```

Successful response (`201 Created`):

```json
{
  "purchase_id": "generated-uuid",
  "product_id": "keyboard-01",
  "purchased_quantity": 2,
  "remaining_quantity": 8,
  "purchased_at": "2026-10-07T12:00:00Z"
}
```

#### `GET /notifications/purchases`

This is a long-lived SSE endpoint. After a committed purchase, connected clients receive an event such as:

```text
event: purchase.completed
data: {"purchase_id":"generated-uuid","product_id":"keyboard-01","quantity":2,"purchased_at":"2026-10-07T12:00:00Z"}
```

Keep the HTTP models independent from generated Protobuf types. The Checkout API explicitly maps between its public JSON contract and the internal gRPC contract.

### Private gRPC API

Suggested contract:

```proto
syntax = "proto3";

package inventory.v1;

service Inventory {
  rpc GetAvailability(GetAvailabilityRequest)
      returns (GetAvailabilityResponse);

  rpc CreatePurchase(CreatePurchaseRequest)
      returns (CreatePurchaseResponse);

  rpc WatchPurchases(WatchPurchasesRequest)
      returns (stream PurchaseEvent);
}

message GetAvailabilityRequest {
  string product_id = 1;
}

message GetAvailabilityResponse {
  string product_id = 1;
  string name = 2;
  uint32 available_quantity = 3;
}

message CreatePurchaseRequest {
  string product_id = 1;
  uint32 quantity = 2;
}

message CreatePurchaseResponse {
  string purchase_id = 1;
  string product_id = 2;
  uint32 purchased_quantity = 3;
  uint32 remaining_quantity = 4;
  string purchased_at = 5;
}

message WatchPurchasesRequest {}

message PurchaseEvent {
  string purchase_id = 1;
  string product_id = 2;
  uint32 quantity = 3;
  string purchased_at = 4;
}
```

`WatchPurchases` is a server-streaming RPC: the Checkout API sends one subscription request, and the Inventory Service sends zero or more events over the same connection.

## SQLite design

The Inventory Service is the only process that accesses the database. Use `sqlx` with its SQLite and Tokio runtime features and a small connection pool.

```sql
CREATE TABLE products (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    stock        INTEGER NOT NULL CHECK (stock >= 0)
);

CREATE TABLE purchases (
    id           TEXT PRIMARY KEY,
    product_id   TEXT NOT NULL REFERENCES products(id),
    quantity     INTEGER NOT NULL CHECK (quantity > 0),
    purchased_at TEXT NOT NULL
);
```

Commit a small migration and seed two or three products at startup with `INSERT OR IGNORE`. Store timestamps as UTC RFC 3339 strings to avoid adding date-handling complexity.

### Atomic purchase rule

`CreatePurchase` must update stock and insert the purchase in one SQLite transaction:

1. Reject a zero quantity before opening the transaction.
2. Decrease stock with a conditional statement such as `UPDATE products SET stock = stock - ? WHERE id = ? AND stock >= ?`.
3. Inspect the affected-row count. If it is zero, distinguish an unknown product from insufficient stock with one follow-up query.
4. Insert the purchase row.
5. Commit the transaction.
6. Only after the commit succeeds, publish `PurchaseEvent` to the in-process broadcast channel.

This ensures concurrent requests cannot make stock negative and subscribers never receive an event for a rolled-back purchase.

## Streaming design

The Inventory Service owns a `tokio::sync::broadcast` sender for newly committed purchase events.

- Each `WatchPurchases` call subscribes to the broadcast channel.
- Tonic adapts that receiver into a server stream.
- The Checkout API runs a background task that maintains the gRPC subscription.
- Received gRPC events are sent into a second local broadcast channel.
- Each public SSE connection subscribes to the Checkout API's local channel.
- SSE sends a periodic keep-alive so idle connections remain visible and testable.

For the two-day version, notifications are **live-only**. A disconnected or slow subscriber can miss events, while the purchase record remains safely stored in SQLite. Durable replay, offsets, and message brokers are explicit non-goals. Log lagged subscribers and reconnect the Checkout API's gRPC stream with a small bounded backoff if the Inventory Service restarts.

## Error mapping

The Inventory Service returns meaningful gRPC statuses, which the Checkout API translates into public HTTP responses.

| Inventory result | gRPC status | Public HTTP result |
|---|---|---|
| Product exists or purchase succeeds | `OK` | `200 OK` or `201 Created` |
| Product is unknown | `NOT_FOUND` | `404 Not Found` |
| Quantity is zero | `INVALID_ARGUMENT` | `400 Bad Request` |
| Not enough stock | `FAILED_PRECONDITION` | `409 Conflict` |
| Internal call exceeds its deadline | `DEADLINE_EXCEEDED` | `504 Gateway Timeout` |
| Inventory Service is unavailable | `UNAVAILABLE` | `503 Service Unavailable` |

Public errors should be safe JSON messages and should not expose certificate details, database queries, internal addresses, or raw server errors.

## Security design

Use **mTLS for every gRPC connection**, including the long-lived streaming connection:

- Create one local development certificate authority (CA).
- Give the Inventory Service its own server certificate and private key.
- Give the Checkout API its own client certificate and private key.
- Configure Tonic's server to trust only client certificates signed by the development CA.
- Configure Tonic's client to trust the CA, verify the Inventory Service hostname, and present its client certificate.
- Put a matching DNS name, such as `inventory.local`, in the server certificate's Subject Alternative Name. For local development, configure the Tonic endpoint's TLS domain name accordingly.
- Keep generated keys, certificates, and `stockguard.db` out of Git. Commit only migrations, a certificate-generation script, and setup instructions.

This provides confidentiality, integrity, server authentication, and client authentication. For the learning version, certificate possession is sufficient authorization. A production version would also require certificate rotation, workload identity, per-method authorization, network policies, and TLS on the public API. Local REST and SSE may use plain HTTP because this exercise specifically secures the service-to-service boundary.

## Suggested Rust stack

| Concern | Crate/tool |
|---|---|
| Async runtime | `tokio` |
| Public REST and SSE | `axum` |
| gRPC client and server | `tonic` with `transport` and a TLS feature |
| Protobuf messages | `prost`, `tonic-prost`, `tonic-prost-build` |
| SQLite | `sqlx` with SQLite and Tokio runtime features |
| JSON | `serde`, `serde_json` |
| IDs | `uuid` |
| UTC timestamps | `time` or `chrono` |
| Stream adapters | `tokio-stream` |
| Logging/tracing | `tracing`, `tracing-subscriber` |
| Certificate generation | a small `openssl` script |

Use mutually compatible versions from the Tonic family. The existing `index.html` guide targets the Tonic 0.14 API family and includes examples for Protobuf generation, streaming, deadlines, error statuses, and TLS configuration.

## Proposed repository layout

```text
.
├── Cargo.toml
├── build.rs
├── migrations/
│   └── 0001_create_inventory.sql
├── proto/
│   └── inventory.proto
├── scripts/
│   └── generate-certs.sh
├── certs/                         # generated and git-ignored
├── data/                          # SQLite file and git-ignored
├── src/
│   ├── lib.rs                     # generated proto module
│   ├── bin/
│   │   ├── checkout_api.rs
│   │   └── inventory_service.rs
│   ├── checkout/
│   │   ├── handlers.rs
│   │   ├── notifications.rs
│   │   └── error.rs
│   └── inventory/
│       ├── repository.rs
│       └── service.rs
└── tests/
    └── purchase_flow.rs            # optional if time permits
```

## Two-day implementation plan

### Day 1 — Persistence and request flow

Target: a complete purchase round trip over unsecured local gRPC.

1. Add dependencies, the Protobuf build script, and the SQLite migration.
2. Define the three RPCs and confirm generated code compiles.
3. Create the SQLite pool, run migrations, and seed sample products.
4. Implement `GetAvailability` and transactional `CreatePurchase`.
5. Implement the Checkout API REST handlers and gRPC-to-HTTP error mapping.
6. Manually test successful, missing-product, invalid-quantity, insufficient-stock, and concurrent-purchase cases.

Suggested stopping point: a REST purchase changes stock and persists a purchase row without allowing negative stock.

### Day 2 — Streaming and security

Target: live purchase notifications and fully authenticated internal traffic.

1. Implement the Inventory Service broadcast channel and `WatchPurchases` stream.
2. Add the Checkout API's background gRPC subscriber and public SSE endpoint.
3. Test the stream in one terminal while creating purchases in another.
4. Add a repeatable local certificate-generation script.
5. Require and verify client certificates on the Inventory Service.
6. Configure both unary and streaming Checkout API calls with the client identity, trusted CA, and expected server domain.
7. Add a two-second deadline to unary calls, reconnect logic for the long-lived stream, and structured logs.
8. Verify the positive and negative mTLS cases and document all run commands.

Suggested stopping point: a valid REST purchase is persisted, produces a live SSE notification, and fails when the Checkout API does not have a trusted client certificate.

## Acceptance criteria

- Both servers run as separate processes.
- The Inventory Service is the only owner of the SQLite database.
- `GET /products/{id}/availability` obtains its data through gRPC.
- `POST /purchases` updates stock and inserts a purchase in one transaction.
- Concurrent successful purchases cannot make stock negative.
- A committed purchase produces one live gRPC `PurchaseEvent`.
- `GET /notifications/purchases` forwards live purchase events as SSE.
- A failed or rolled-back purchase does not produce an event.
- The unary and streaming gRPC connections verify the Inventory Service certificate.
- The Inventory Service requires and verifies the Checkout API's client certificate.
- Expected gRPC statuses are translated into appropriate HTTP responses.
- Unary internal calls have a deadline, and the streaming client reconnects after interruption.
- The repository contains reproducible setup instructions but no private keys or database files.

## Test checklist

Manual tests are sufficient for the two-day scope:

1. Query an existing product and receive its stock from SQLite.
2. Query an unknown product and receive HTTP 404.
3. Purchase valid stock and confirm both the product and purchase rows changed.
4. Purchase zero items and receive HTTP 400.
5. Purchase more than is available and receive HTTP 409 with no database changes.
6. Run two competing purchases and confirm the transaction prevents negative stock.
7. Open the SSE endpoint, make a purchase, and receive one notification after commit.
8. Make a rejected purchase and confirm no notification appears.
9. Restart the Inventory Service and confirm the Checkout API reconnects its stream.
10. Omit the client certificate and confirm the Inventory Service rejects the connection.
11. Use a certificate signed by another CA and confirm the connection is rejected.
12. Configure the wrong server domain and confirm hostname verification fails.

If time remains, automate the business cases as service-level tests and keep the TLS failure cases as an executable smoke-test script.

## Explicit non-goals

To protect the two-day schedule, do not add:

- User accounts, OAuth, or JWT validation
- A graphical frontend
- Kubernetes, Docker Compose, or cloud deployment
- Actual payment-provider integration
- Multiple Inventory Service replicas
- Durable notification replay or delivery guarantees
- Kafka, RabbitMQ, Redis, or another message broker
- Client-streaming or bidirectional-streaming RPCs
- Production certificate issuance or automatic rotation

These are useful follow-up projects, but none is required to learn the intended architecture.

## Optional stretch goals

Choose at most one after the acceptance criteria pass:

1. **Idempotency:** require an idempotency key so retrying `POST /purchases` does not reduce stock twice or publish duplicate events.
2. **Purchase history:** expose `GET /purchases` through the Checkout API.
3. **Integration test:** start both servers on ephemeral ports and test the full REST-to-gRPC-to-SSE path.
4. **Certificate identity check:** inspect the authenticated client certificate and allow only the expected Checkout API identity.

Idempotency is the most valuable real-world extension because a timeout does not reveal whether the purchase transaction committed.

## Definition of done

The project is done when documented commands start both servers, SQLite persists purchases, example REST calls behave as specified, a committed purchase appears on the live SSE stream, and negative mTLS tests prove that an untrusted client cannot use the internal service. Further production features should be separate iterations rather than additions to this two-day scope.
