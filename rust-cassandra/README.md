# ParkWatch Mall

Smart parking on a 3-node Cassandra cluster. See [PROJECT.md](PROJECT.md) for the plan.

## Layout

```
Cargo.toml          workspace: parkd + sim
parkd/              axum HTTP service (device ingest, driver and operator queries)
sim/                simulated spot units and gate cameras
Dockerfile          one image containing both binaries
docker-compose.yml  cassandra1..3, parkd, sim
```

## Run

```sh
docker compose up -d --build          # nodes join one at a time, ~2-3 minutes
docker compose ps                     # wait for all 3 nodes to be "healthy"
docker compose exec cassandra1 nodetool status   # expect 3 lines starting with UN
curl -s localhost:8080/health         # nodes the driver discovered
docker compose logs -f sim            # the running mall: cars, lights, faults
curl -s localhost:8080/levels/free    # B1/B2/B3 availability
docker compose exec cassandra1 cqlsh  # CQL shell
```

After changing Rust code: `docker compose up -d --build parkd sim`.

Stop with `docker compose down`. Add `-v` to also delete the Cassandra data volumes.

Cassandra is only reachable inside the compose network, because the driver
connects to every node by its Docker-internal IP, and those IPs don't route
from macOS. That is why parkd and sim run as compose services.

## Phases 1–2

Compose runs the idempotent `parkd --init-schema` job before starting the API.
It creates the `parkwatch` keyspace (RF 3 in `dc1`) and all project tables.
Run it manually after a schema change with:

```sh
docker compose run --rm schema
```

`sim --seed` creates three levels with 40 spots each through parkd's internal
bootstrap endpoint. `sim --timestamps` seeds, then posts 60 spot events:
10% are duplicates and 10% are delivered five seconds late. The late events
have an older device timestamp, so Cassandra retains the newer state.

```sh
docker compose run --rm sim --seed
docker compose run --rm sim --timestamps
curl -s localhost:8080/levels/free
docker compose exec cassandra1 cqlsh -e \
  "SELECT spot_id, occupied, light_reported FROM parkwatch.spots_by_level WHERE level = 'B1';"
```

The `POST /events` payload is tagged by `type`. Device data is written with
the device's timestamp in microseconds (`USING TIMESTAMP`).

## Phases 3–5

The `sim` service now runs a mall until stopped. Cars pass the entry camera,
15% reserve a spot first, park for 30–180 seconds, leave, and pass the exit
camera. Every unit heartbeats every 15 seconds and applies the override in the
reply. On purpose, 10% of events are sent twice, 10% of spot events arrive 5
seconds late, 2 bulbs are broken and 3 units stop heartbeating after 45
seconds. The sim logs which spots it broke, so you can compare.

| Endpoint | What it shows |
|---|---|
| `GET /cars/{plate}` | Where a parked car is |
| `GET /ops/report?day=YYYY-MM-DD` | Visits, average stay, revenue (UTC day, default today) |
| `POST /spots/{id}/reserve` `{"reserved_by": "..."}` | 201 reserved, 409 taken or under maintenance |
| `DELETE /spots/{id}/reserve?reserved_by=...` | 204 cancelled, 403 not yours, 404 none |
| `PUT /spots/{id}/maintenance` `{"on": true, "hours": 2}` | Light off for N hours; `{"on": false}` ends it |
| `GET /ops/offline` | Spots with no heartbeat in the last 60s |
| `GET /ops/faults` | Spots whose reported light differs from the expected one |

### Sessions and fees

```sh
# A car that entered 3h10m ago: 5000 + 3 x 3000 = 14000 cents
now=$(date -u +%Y-%m-%dT%H:%M:%SZ); then=$(date -u -v-190M +%Y-%m-%dT%H:%M:%SZ)
curl -s localhost:8080/events -H 'content-type: application/json' \
  -d "{\"type\":\"gate\",\"gate\":\"entry-1\",\"direction\":\"in\",\"plate\":\"DEMO1\",\"ts\":\"$then\"}"
curl -s localhost:8080/events -H 'content-type: application/json' \
  -d "{\"type\":\"gate\",\"gate\":\"exit-1\",\"direction\":\"out\",\"plate\":\"DEMO1\",\"ts\":\"$now\"}"
curl -s localhost:8080/ops/report
```

### Reservations

```sh
docker compose run --rm sim --race            # 100 drivers, one spot: expect exactly 1 winner
docker compose run --rm sim --race B1-05      # pick another spot

RESERVATION_TTL_SECONDS=20 docker compose up -d parkd   # quick TTL test
curl -s -XPOST localhost:8080/spots/B3-39/reserve -H 'content-type: application/json' -d '{"reserved_by":"me"}'
curl -s localhost:8080/levels/free            # one fewer free spot; ~20s later it's back
docker compose up -d parkd                    # back to 900s
```

### Maintenance and faults

```sh
curl -s -XPUT localhost:8080/spots/B2-10/maintenance -H 'content-type: application/json' -d '{"on":true,"hours":0.5}'
docker compose logs sim | grep B2-10          # light turns off within one heartbeat (15s)
curl -s localhost:8080/ops/offline
curl -s localhost:8080/ops/faults
```
