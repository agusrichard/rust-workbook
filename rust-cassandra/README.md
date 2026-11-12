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
docker compose logs sim               # seed + duplicate/late event run
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
bootstrap endpoint. Normal `sim` runs seed first, then posts 60 spot events:
10% are duplicates and 10% are delivered five seconds late. The late events
have an older device timestamp, so Cassandra retains the newer state.

```sh
docker compose run --rm sim --seed
curl -s localhost:8080/levels/free
docker compose exec cassandra1 cqlsh -e \
  "SELECT spot_id, occupied, light_reported FROM parkwatch.spots_by_level WHERE level = 'B1';"
```

The `POST /events` payload is tagged by `type`; Phase 2 persists `occupied`
and `free` events using a device timestamp in microseconds. `heartbeat` and
`gate` payloads are accepted now and gain persistence in Phases 5 and 3.
