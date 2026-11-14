# Project: ParkWatch Mall, smart parking for one mall or building

## 1. Project idea

A mall has a car park with several levels (B1, B2, B3). There are two kinds of IoT devices:

- **Spot unit:** mounted above each parking spot. It has a car sensor, a camera that reads license plates, and a light.
- **Gate camera:** at the entry and exit gates. It reads the license plate of every car that comes in or goes out.

The light above each spot tells drivers the spot's status from a distance:

| Color | Meaning | Who decides |
|---|---|---|
| 🟢 Green | Free | The spot unit itself |
| 🔴 Red | Taken | The spot unit itself |
| 🔵 Blue | Reserved | The server (an override) |
| ⚫ Off | Under maintenance | The server (an override) |

Features:

1. **Free spots per level:** screens at the entrance show something like "B1: 12 free, B2: 40 free".
2. **Entry and exit:** the entry gate starts a parking session. The exit gate ends it and calculates the fee.
3. **Find my car:** a driver types their plate and gets the level and spot, like "B2, spot 17".
4. **Reserve a spot:** the spot is held for 15 minutes and its light turns blue.
5. **Maintenance:** an operator turns a spot off so nobody parks there.
6. **Operator view:** live occupancy, offline devices, broken lights, and a daily report on visits, average stay and revenue.

You'll build it in Rust against a **3-node Cassandra cluster** run with Docker Compose.

What you learn:

| Problem | Cassandra concept |
|---|---|
| Devices send duplicate and late events | `USING TIMESTAMP`: the device's clock decides which write wins |
| Two drivers reserve the same spot | Lightweight transactions (LWT) with `IF NOT EXISTS` |
| A reservation or maintenance window should end on its own | TTL |
| A device that goes silent should be flagged | Heartbeat rows with a TTL ("no row" means offline) |
| A car must not have two open sessions | LWT on the session table |
| Level name and capacity are shared by every spot on a level | Static columns |
| One mall has only a few levels | Hot partitions |
| Nodes fail | Consistency levels, hinted handoff, repair |

## 2. High-level design

### Components

```
 ┌───────────────────── docker compose network ──────────────────────┐
 │                                                                    │
 │  [sim]  ──HTTP──►  [parkd]  ──CQL──►  cassandra1 ◄─► cassandra2    │
 │  spot units +      axum API              ▲      gossip      ▲      │
 │  gate cameras      (ingest +             └──── cassandra3 ──┘      │
 │  (simulated)        queries)                                       │
 │                       ▲                                            │
 │  [you: curl] ─────────┘  drivers, entrance screens, operators      │
 └────────────────────────────────────────────────────────────────────┘
```

Use a Cargo workspace with two binaries:

- **`parkd`**: the axum HTTP service. It takes in device events and answers driver and operator queries.
- **`sim`**: simulates every spot unit and both gate cameras. Cars arrive, park, and leave at random. On purpose, it sends some events twice, sends some late, stops some devices' heartbeats, and gives a few spots a "broken bulb".

### How the light works

The **spot unit switches red and green itself**, the moment its sensor sees a change. The light keeps working even if the server or Cassandra is down. The unit then reports what it did.

The **server only sets overrides** (blue or off). The unit learns about an override in the reply to its heartbeat, which it sends every 15 seconds.

Light rules on the device, in priority order:

```
1. override = off          → OFF   (maintenance, always wins)
2. a car is in the spot    → RED   (a car in a reserved spot still shows red)
3. override = blue         → BLUE
4. otherwise               → GREEN
```

### Device events

```json
{ "type": "occupied",  "spot": "B2-17", "plate": "B1234XY", "light": "red",   "ts": "..." }
{ "type": "free",      "spot": "B2-17",                     "light": "green", "ts": "..." }
{ "type": "heartbeat", "spot": "B2-17", "light": "green", "battery": 87,      "ts": "..." }
{ "type": "gate",      "gate": "entry-1", "direction": "in", "plate": "B1234XY", "ts": "..." }
```

The reply to a heartbeat contains the override: `{ "override": null | "blue" | "off" }`.

### Business and logic flow

```
CAR ENTERS
  entry gate event → INSERT INTO active_session_by_plate ... IF NOT EXISTS
                     (a duplicate camera read does nothing, and a car can't have 2 open sessions)

CAR PARKS
  unit turns RED   → "occupied" event
                   → write spots_by_level  USING TIMESTAMP <event ts>
                   → write car_location_by_plate (for "find my car")
                   → write spot_events_by_spot_day (history)
                   → DELETE FROM reservations WHERE spot_id=? IF EXISTS
                     → if applied: clear the blue override (the reserved car has arrived)

CAR LEAVES THE SPOT
  unit turns GREEN → "free" event → write spots_by_level and the history (USING TIMESTAMP)

CAR EXITS
  exit gate event  → read active_session_by_plate → calculate the fee
                   → write sessions_by_day
                   → DELETE FROM active_session_by_plate ... IF EXISTS

DRIVER RESERVES A SPOT
  → check the spot is free and not under maintenance
  → INSERT INTO reservations ... IF NOT EXISTS USING TTL 900
  → if applied: UPDATE spots_by_level USING TTL 900 SET light_override = 'blue'
  → otherwise: 409 Conflict ("someone got it first")
  (after 15 minutes both rows expire and the light goes back to green on its own)

OPERATOR SETS MAINTENANCE
  → UPDATE spots_by_level USING TTL <hours * 3600> SET light_override = 'off'
  → ending it early: SET light_override = null

DEVICE HEARTBEAT
  → write sensor_heartbeats USING TTL 60
  → write light_reported (USING TIMESTAMP)
  → reply with the spot's current light_override
```

### Tables

| Table | Partition key | Clustering | Purpose |
|---|---|---|---|
| `spots_by_level` | `level` | `spot_id` | Spot state: `occupied`, `plate`, `light_reported`, `light_override`. **Static** `level_name` and `capacity`. Answers "free spots per level" and the faults check |
| `reservations` | `spot_id` | none | Reservations, written **only with LWT**, TTL 900 |
| `active_session_by_plate` | `plate` | none | Open parking sessions, written **only with LWT** |
| `car_location_by_plate` | `plate` | none | "Find my car", TTL 24h |
| `sessions_by_day` | `day` | `exited_at, plate` | Closed sessions with duration and fee, for the daily report |
| `spot_events_by_spot_day` | `(spot_id, day)` | `ts DESC` | History of one spot, TTL 30 days |
| `sensor_heartbeats` | `level` | `spot_id` | Battery and last seen, TTL 60s, so a missing row means offline |

Design notes:

- **The LWT tables (`reservations`, `active_session_by_plate`) only ever get LWT writes.** Mixing plain writes and LWT writes on the same data breaks the guarantee LWT gives you. That is why the car's location lives in its own table and isn't added to the session row.
- **Device data uses the device's clock and server data uses the server's clock, in separate columns.** `occupied`, `plate` and `light_reported` are written `USING TIMESTAMP <event ts>`. `light_override` is written with normal (server) timestamps. Mixing the two clocks on one column would let one clock silently override the other.
- **The partitions per level are hot.** A mall with 3 levels has only 3 partitions in `spots_by_level`, so every event lands on one of 3 partitions. That is fine at mall scale, and Phase 7 measures it.
- **Store the fee in cents (`int`)**, never as a floating-point number. Example rule: the first hour costs 5000, and each further hour, or part of an hour, costs 3000.

### API

| Method and path | Used by |
|---|---|
| `POST /events` | Spot units and gate cameras |
| `GET /levels/free` | Entrance screens |
| `GET /cars/{plate}` | Drivers ("find my car") |
| `POST /spots/{id}/reserve`, `DELETE /spots/{id}/reserve` | Drivers |
| `PUT /spots/{id}/maintenance` `{ "on": true, "hours": 2 }` | Operators |
| `GET /ops/offline`, `GET /ops/faults`, `GET /ops/report?day=` | Operators |

## 3. Build checklist

### Phase 0: Set up the cluster and project

- [x] **Write `docker-compose.yml` with 3 nodes** (`cassandra:5.0`). Use the same `CASSANDRA_CLUSTER_NAME` on all 3, set `CASSANDRA_SEEDS=cassandra1`, and set `CASSANDRA_DC=dc1` with `CASSANDRA_ENDPOINT_SNITCH=GossipingPropertyFileSnitch`. Set `MAX_HEAP_SIZE=512M` and `HEAP_NEWSIZE=128M`.
  - *Concept, seeds and gossip:* a seed is just the address a new node contacts to find the cluster. After joining, nodes keep each other up to date by gossiping every second.
- [x] **Start the nodes one at a time,** using healthchecks with `depends_on: condition: service_healthy`.
  - *Concept, bootstrapping:* a joining node claims token ranges and copies data from the other nodes. Two nodes joining at the same moment can conflict.
- [x] **Check the cluster:** `nodetool status` should show 3 nodes marked `UN` (Up/Normal).
- [x] **Create a Cargo workspace** with `parkd` and `sim`. Add `scylla`, `tokio`, `axum`, `serde`, `chrono`, `reqwest` (for `sim`), `rand` and `anyhow`.
- [x] **Run `parkd` and `sim` as compose services.** The driver discovers the other nodes by their Docker-internal IPs, which your Mac can't reach, so the app has to run inside the network.

### Phase 1: Set up the schema

- [x] **Add `parkd --init-schema`** to create keyspace `parkwatch` with `NetworkTopologyStrategy {'dc1': 3}` and all the tables. Use `IF NOT EXISTS` so it's safe to run twice.
  - *Concept, replication factor:* RF=3 means 3 copies, so you can lose 1 node and still use `QUORUM`.
- [x] **Give `spots_by_level` its static columns,** `level_name text STATIC, capacity int STATIC`.
  - *Concept, static columns:* the value is stored once per partition and appears on every row in it.
- [x] **Add `sim --seed`** to create the spots, for example 3 levels × 40 spots.

### Phase 2: Spot events and lights

- [x] **Add `POST /events`** and turn the JSON into a Rust enum of event types.
- [x] **Write spot state with `UPDATE spots_by_level USING TIMESTAMP ? SET occupied = ?, plate = ?, light_reported = ? ...`.** The timestamp is in microseconds.
  - *Concept, client-side timestamps:* every replica keeps the value with the highest timestamp. A duplicate rewrites the same value, and a late event loses to newer data. That makes ingestion idempotent without reading before writing.
- [x] **Implement the light rules in `sim`** (off, then red, then blue, then green). The unit reports its light color in every event.
- [x] **Have `sim` duplicate 10% of events and send 10% of them 5 seconds late.** Check that the final state in Cassandra matches what the sim logged. Then remove `USING TIMESTAMP` and watch the state go wrong.
- [x] **Add `GET /levels/free`:** read the 3 level partitions concurrently and count the free spots that have no override.

### Phase 3: Gates, sessions and fees

- [x] **Entry:** write `INSERT INTO active_session_by_plate ... IF NOT EXISTS` and read `[applied]`.
  - *Concept, LWT:* Paxos runs among the replicas so the check and the write happen as one step. It costs about 4 round trips. A duplicate gate read becomes `[applied] = false`, and nothing breaks.
- [x] **Exit:** read the session, calculate the fee, write `sessions_by_day`, then `DELETE ... IF EXISTS`.
- [x] **Find my car:** occupied events write `car_location_by_plate`, and `GET /cars/{plate}` reads it.
- [x] **Add `GET /ops/report?day=`** returning visits, average stay and revenue from one partition of `sessions_by_day`.
  - *Concept:* the report reads a single partition, so it's fast. A report over a month means 30 partitions read concurrently, never a table scan.

### Phase 4: Reservations (blue light)

- [x] **Add `POST /spots/{id}/reserve`:** `INSERT INTO reservations ... IF NOT EXISTS USING TTL 900`, and if applied, set `light_override = 'blue'` with the same TTL.
  - *Concept, TTL as a timer:* both rows expire together, and the light goes back to green with no cleanup job.
- [x] **Add cancelling:** `DELETE FROM reservations WHERE spot_id = ? IF reserved_by = ?`, so only the holder can cancel. Then clear the override.
- [x] **When the car arrives:** the occupied event deletes the reservation with `IF EXISTS`, and clears the blue override if that delete was applied.
- [x] **Race test:** fire 100 concurrent reserve requests for the same spot. **Exactly 1** should succeed. Repeat without `IF NOT EXISTS` and see that everyone "wins".
- [x] **Test the TTL quickly** by setting it to 20 seconds and watching the reservation expire.

### Phase 5: Maintenance (light off) and faults

- [x] **Add `PUT /spots/{id}/maintenance`:** set `light_override = 'off'` with `USING TTL <hours>`, or set it to null to end maintenance early. Reject reservations for spots under maintenance.
- [x] **Return the override in the heartbeat reply.** Check that the sim's light changes within one heartbeat.
- [x] **Offline devices:** write heartbeats with `USING TTL 60`. `GET /ops/offline` lists the spots on each level that have no heartbeat row.
- [x] **Broken lights:** `GET /ops/faults` compares the expected color (worked out from `occupied` and `light_override`) with `light_reported`. Have `sim` break a bulb on purpose and see it appear here.
  - *Concept:* the server stores **what it wants** (`light_override`) and **what the device reports** (`light_reported`) separately. Comparing the two is a common IoT pattern for finding faults.

### Phase 6: Multi-node experiments

- [x] **Find where data lives:** `nodetool getendpoints parkwatch spots_by_level 'B2'` shows which nodes hold level B2.
- [x] **Trace a query:** in `cqlsh`, run `TRACING ON;` and then a `SELECT`. The trace shows the coordinator, the replicas it contacted, and timings.
- [x] **Stop 1 node** (`docker compose stop cassandra3`). Everything still works: `QUORUM` reads and writes, and LWT reservations and sessions.
- [x] **Stop 2 nodes.** Spot events at consistency `ONE` still get written, so lights and occupancy keep updating. Reservations and gate sessions fail, because LWT needs a quorum.
  - *Concept:* decide which features must be strict (money and reservations) and which can be relaxed (occupancy).
- [x] **Watch hinted handoff:** stop node3, keep the sim running for a minute, and run `nodetool listpendinghints` on node1. Start node3 again and watch the hints drain.
- [x] **Repair:** run `nodetool repair parkwatch`, then check that node3 has the same spot states as the others.
  - *Concept:* hints, read repair and full repair are the 3 ways Cassandra brings out-of-date copies back in sync.

### Phase 7: Stretch goals

- [x] **Measure the hot partition:** run the sim at 10× speed and check `nodetool tablestats parkwatch.spots_by_level`. Then try splitting the partition into `(level, zone)` and compare.
- [x] **Counters:** add a `free_count_by_level` counter, kill a node while it's being updated, and compare it with the real count. Counter updates can't be safely retried, so the count drifts.
- [x] **Live map:** a small HTML page served by `parkd` that shows every spot as a colored square and refreshes every 2 seconds.
- [x] **Monthly pass:** registered plates get free parking, using a `passes_by_plate` table checked at the exit gate.

### Suggested order

Do Phases 0–2 first. They cover the core of IoT ingestion and the lights. Phases 3–5 add the business features. Phase 6 is where the multi-node lessons happen, so give it a full session.
