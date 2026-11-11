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
docker compose logs sim               # sim reached parkd
docker compose exec cassandra1 cqlsh  # CQL shell
```

After changing Rust code: `docker compose up -d --build parkd sim`.

Stop with `docker compose down`. Add `-v` to also delete the Cassandra data volumes.

Cassandra is only reachable inside the compose network, because the driver
connects to every node by its Docker-internal IP, and those IPs don't route
from macOS. That is why parkd and sim run as compose services.
