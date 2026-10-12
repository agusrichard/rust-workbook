# StockGuard

StockGuard is a small Rust learning project with two independently running servers:

- **Checkout API** exposes public REST and Server-Sent Events (SSE) with Axum.
- **Inventory Service** owns SQLite and exposes private unary and server-streaming gRPC with Tonic.

Every Checkout-to-Inventory call uses mutual TLS (mTLS). A purchase reduces stock and inserts a purchase in one transaction; only a committed purchase becomes a live notification.

## Request flow

```text
Public client                    Checkout API                 Inventory Service
     |                         HTTP :8080                    gRPC :50051
     | GET /products/{id}          |--- GetProduct (mTLS) -------->|
     |<---------- JSON ------------|<-----------------------------|
     |                              |                              |
     | POST /purchases              |--- CreatePurchase (mTLS) --->|--- SQLite transaction
     |<---------- JSON ------------|<-----------------------------|
     |                              |                              |
     | GET /notifications/purchases|<-- WatchPurchases (mTLS) ----|--- committed event
     |<----------- SSE ------------|                              |
```

Notifications are live-only. A client that connects after a purchase will not receive that historical event.

## Prerequisites

- Rust and Cargo
- `protoc`
- SQLite's `sqlite3` CLI (useful for inspection)
- OpenSSL
- `curl`

Check the tools:

```sh
rustc --version
cargo --version
protoc --version
sqlite3 --version
openssl version
curl --version
```

## Initial setup

Generate local development identities and create the database directory:

```sh
./scripts/generate_certs.sh
mkdir -p data
```

The certificate script creates one development CA, an Inventory server identity for `inventory.local`, and a distinct Checkout client identity. The generated `certs/` directory contains development credentials only and is ignored by Git. Do not use these credentials in production.

The database is created, migrated, and seeded automatically when Inventory starts. To start over locally, stop both services and remove `data/stockguard.db`.

## Run the services

Start Inventory in the first terminal:

```sh
RUST_LOG=info cargo run --bin inventory_service
```

Start Checkout in the second terminal:

```sh
RUST_LOG=info cargo run --bin checkout_api
```

Inventory listens on `[::1]:50051`; Checkout listens on `127.0.0.1:8080`. Press Ctrl+C to shut either service down gracefully.

## Try the API

Retrieve a seeded product:

```sh
curl -i http://127.0.0.1:8080/products/1
```

Open the live notification stream in another terminal before making a purchase:

```sh
curl -N http://127.0.0.1:8080/notifications/purchases
```

Create a purchase:

```sh
curl -i \
  -H 'content-type: application/json' \
  -d '{"product_id":1,"quantity":2}' \
  http://127.0.0.1:8080/purchases
```

The purchase returns `201 Created`. The connected SSE client receives a `purchase.completed` event with the same purchase ID, product ID, quantity, and timestamp.

Try expected failures:

```sh
# Unknown product: 404 Not Found
curl -i http://127.0.0.1:8080/products/999

# Invalid quantity: 400 Bad Request
curl -i \
  -H 'content-type: application/json' \
  -d '{"product_id":1,"quantity":0}' \
  http://127.0.0.1:8080/purchases

# Insufficient stock: 409 Conflict
curl -i \
  -H 'content-type: application/json' \
  -d '{"product_id":1,"quantity":999}' \
  http://127.0.0.1:8080/purchases
```

Inspect persisted data:

```sh
sqlite3 data/stockguard.db \
  'SELECT id, name, stock FROM products; SELECT id, product_id, quantity, purchased_at FROM purchases;'
```

## Configuration

All settings are optional and have local defaults.

| Variable | Used by | Default |
|---|---|---|
| `DATABASE_URL` | Inventory | `sqlite://data/stockguard.db` |
| `INVENTORY_GRPC_ADDRESS` | Inventory | `[::1]:50051` |
| `CHECKOUT_HTTP_ADDRESS` | Checkout | `127.0.0.1:8080` |
| `INVENTORY_GRPC_ENDPOINT` | Checkout | `https://[::1]:50051` |
| `INVENTORY_TLS_DOMAIN` | Checkout | `inventory.local` |
| `TLS_CA_CERT` | Both | `certs/ca.pem` |
| `INVENTORY_TLS_CERT` | Inventory | `certs/inventory.pem` |
| `INVENTORY_TLS_KEY` | Inventory | `certs/inventory-key.pem` |
| `CHECKOUT_TLS_CERT` | Checkout | `certs/checkout.pem` |
| `CHECKOUT_TLS_KEY` | Checkout | `certs/checkout-key.pem` |
| `RUST_LOG` | Both | `info` |

The TLS domain must match the DNS Subject Alternative Name in the Inventory certificate even when the endpoint uses the IPv6 loopback address.

## Verify the project

```sh
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
```

The automated tests use isolated SQLx test databases rather than `data/stockguard.db`. Manual acceptance should cover REST, SSE, restart/reconnection, and the negative mTLS cases listed in [PROJECT_PROPOSAL.md](PROJECT_PROPOSAL.md).

## Design boundaries

- Only Inventory accesses SQLite.
- Checkout's public JSON models are separate from generated Protobuf models.
- Unary gRPC calls have a two-second deadline; the deliberate long-lived subscription does not.
- Broadcast channels are bounded. Lag is logged, but there is no durable replay.
- Detailed internal failures stay in structured logs; public failures use safe JSON.

The main limitation is that retrying a purchase after an ambiguous timeout can create a second purchase. The most useful next improvement is an idempotency key stored with each purchase, so a repeated request returns the original outcome without reducing stock twice or emitting a duplicate event.
