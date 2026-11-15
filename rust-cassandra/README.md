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
docker compose run --rm sim --race-without-lwt  # read-then-insert: many "winners", one row

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

## Phase 6: multi-node experiments

Each feature picks its consistency where its query is written
(`parkd/src/db/mod.rs`):

| Feature | Consistency | With 2 of 3 nodes down |
|---|---|---|
| Spot events, heartbeats, lights, history, find my car, free spots | `ONE` | keeps working |
| Overrides, sessions, report, passes | `QUORUM` | 503 |
| Reservations, gate sessions | LWT (`SERIAL` + `QUORUM`) | 503 |

An occupied event still tries to claim a reservation, but best-effort: if
the LWT fails, the reservation and its blue light expire by TTL (the spot
shows red anyway).

```sh
# Where B2 lives (RF 3 on 3 nodes: every node), and node IPs
docker compose exec cassandra1 nodetool getendpoints parkwatch spots_by_level B2
for n in 1 2 3; do docker compose exec cassandra$n hostname -i; done

# Trace: a QUORUM read asks 2 replicas; an LWT does 4 round trips to all 3
docker compose exec cassandra1 cqlsh -e "CONSISTENCY QUORUM; TRACING ON;
  SELECT * FROM parkwatch.spots_by_level WHERE level = 'B2' AND spot_id = 'B2-17';"

# Node failures
docker compose stop cassandra3                 # everything works
docker compose stop cassandra2                 # gates/reservations 503, lights fine
docker compose start cassandra2 cassandra3

# Hinted handoff: hint files appear while a node is down and drain on restart
docker compose stop cassandra3; sleep 60
docker compose exec cassandra1 ls -la /var/lib/cassandra/hints
docker compose start cassandra3
docker compose logs -f cassandra1 | grep 'Finished hinted handoff'

# Repair: make node3 miss data for real, then fix it
for n in 1 2; do docker compose exec cassandra$n nodetool disablehandoff; done
docker compose stop cassandra3; sleep 60; docker compose stop sim
docker compose start cassandra3
docker compose exec cassandra3 nodetool repair -full parkwatch
for n in 1 2; do docker compose exec cassandra$n nodetool enablehandoff; done
```

What happened when this was run:

- **1 node down:** 0 errors. Reserve, cancel, entry, exit with fee, report.
- **2 nodes down:** occupied/free/heartbeat/find-my-car/free spots all
  answered. Gates, reservations, maintenance and the report got 503;
  `cqlsh` shows the real reason, `Cannot achieve consistency level QUORUM
  ... required_replicas: 2, alive_replicas: 1`.
- **Hints:** ~95 KB of hints per live node after a minute; delivered and
  deleted 30 s after node3 started. `nodetool listpendinghints` said "no
  pending hints" while the files were growing, so look at the hints
  directory or the `Finished hinted handoff` log line instead.
- **Repair:** with handoff off, 53 of 120 spot rows differed between node1's
  and node3's local copies (read at `ONE`, traced to confirm no other replica
  was asked). `nodetool repair -full` took 3 s; afterwards 0 differed.

## Phase 7: stretch goals

| Endpoint | What it does |
|---|---|
| `GET /map` | Live map: every spot as a square, refreshed every 2 s |
| `GET /spots` | The JSON behind the map (reported vs expected light, online) |
| `GET /ops/counters` | `free_count_by_level` counter next to the real count |
| `PUT /passes/{plate}` `{"holder": "...", "days": 30}` | Monthly pass, free exit while valid |
| `GET`/`DELETE /passes/{plate}` | Look up or revoke a pass |

```sh
open http://localhost:8080/map

# Hot partition: 10x sim, zone table on, compare writes per partition
SPOTS_BY_ZONE=on docker compose up -d parkd
SIM_SPEED=10 docker compose up -d sim
docker compose exec cassandra1 nodetool toppartitions -a WRITES parkwatch spots_by_level 20000
docker compose exec cassandra1 nodetool toppartitions -a WRITES parkwatch spots_by_zone 20000

# Counters: drift with duplicates, none without, then kill a node
curl -s localhost:8080/ops/counters
SIM_DUPLICATE_RATE=0 SIM_LATE_RATE=0 SIM_SPEED=10 docker compose up -d sim
docker compose kill cassandra2; curl -s localhost:8080/ops/counters
docker compose start cassandra2

docker compose up -d parkd sim                 # back to defaults
```

What happened when this was run:

- **Hot partition:** at 10x, each level partition took ~600 writes in 20 s
  (~30/s); each of the 12 zone partitions ~150 (~7.5/s). Local write latency
  was 0.024 ms and the biggest partition 2.7 KB, so at mall scale the level
  partition is not a problem. Splitting would cost the entrance screen 12
  reads instead of 3. It starts to matter at thousands of spots per level.
- **Counters:** with the sim's 10% duplicates and late events, the counter was
  off by -6/-1/+2 within 3 minutes. With those off, it matched exactly for a
  minute at 10x. Killing a node caused 78 counter writes to time out with
  `received: 0` before the failure detector noticed; the drift went up to +6
  while the sim retried, then back to 0, because the dead node had applied
  none of them. Had a timed-out write landed (a slow, not dead, replica), the
  retry would have counted it twice, and nothing can tell.
- **Passes:** a 3h10m stay costs 0 with a pass and 14000 without. The row has
  a 30-day TTL. The sim gives 5% of cars a pass.
