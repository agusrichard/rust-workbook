//! Everything that talks to Cassandra.
//!
//! Each submodule owns one group of tables:
//! - `spots` owns `spots_by_level`, `spot_events_by_spot_day`, `car_location_by_plate`
//! - `sessions` owns `active_session_by_plate` (LWT only) and `sessions_by_day`
//! - `reservations` owns `reservations` (LWT only)
//! - `heartbeats` owns `sensor_heartbeats`
//! - `counters` owns `free_count_by_level`
//! - `passes` owns `passes_by_plate`
//!
//! Every statement goes through [`relaxed`] (ONE), [`strict`] (QUORUM) or
//! [`execute_lwt`] (SERIAL + QUORUM), so the consistency of each feature is
//! visible where the query is written.

pub mod counters;
pub mod heartbeats;
pub mod passes;
pub mod reservations;
pub mod sessions;
pub mod spots;

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use scylla::client::caching_session::CachingSession;
use scylla::client::session::Session;
use scylla::client::session_builder::SessionBuilder;
use scylla::errors::{DbError, ExecutionError, RequestAttemptError, WriteType};
use scylla::response::query_result::QueryResult;
use scylla::serialize::row::SerializeRow;
use scylla::statement::unprepared::Statement;
use scylla::statement::{Consistency, SerialConsistency};
use scylla::value::{CqlValue, Row};
use serde::Serialize;

pub const LEVELS: [&str; 3] = ["B1", "B2", "B3"];

/// The session handlers use. It prepares each distinct CQL string once and
/// reuses the prepared statement, so a request costs one round trip instead
/// of prepare + execute. Prepared statements also let the driver route each
/// request straight to a replica that owns the partition (token awareness).
pub type Db = CachingSession;

/// Creates the complete ParkWatch data model. Every statement is deliberately
/// idempotent so `parkd --init-schema` is safe to repeat.
pub async fn init_schema(session: &Session) -> Result<()> {
    const DDL: [&str; 11] = [
        "CREATE KEYSPACE IF NOT EXISTS parkwatch WITH replication = {'class': 'NetworkTopologyStrategy', 'dc1': 3}",
        "CREATE TABLE IF NOT EXISTS parkwatch.spots_by_level (level text, spot_id text, level_name text STATIC, capacity int STATIC, occupied boolean, plate text, light_reported text, light_override text, PRIMARY KEY (level, spot_id))",
        "CREATE TABLE IF NOT EXISTS parkwatch.reservations (spot_id text PRIMARY KEY, reserved_by text, reserved_at timestamp)",
        "CREATE TABLE IF NOT EXISTS parkwatch.active_session_by_plate (plate text PRIMARY KEY, entered_at timestamp, entry_gate text)",
        "CREATE TABLE IF NOT EXISTS parkwatch.car_location_by_plate (plate text PRIMARY KEY, level text, spot_id text, seen_at timestamp)",
        "CREATE TABLE IF NOT EXISTS parkwatch.sessions_by_day (day date, exited_at timestamp, plate text, entered_at timestamp, duration_seconds bigint, fee_cents int, PRIMARY KEY (day, exited_at, plate)) WITH CLUSTERING ORDER BY (exited_at DESC)",
        "CREATE TABLE IF NOT EXISTS parkwatch.spot_events_by_spot_day (spot_id text, day date, ts timestamp, event_type text, plate text, light text, PRIMARY KEY ((spot_id, day), ts)) WITH CLUSTERING ORDER BY (ts DESC) AND default_time_to_live = 2592000",
        "CREATE TABLE IF NOT EXISTS parkwatch.sensor_heartbeats (level text, spot_id text, battery int, light text, last_seen timestamp, PRIMARY KEY (level, spot_id)) WITH default_time_to_live = 60",
        // Phase 7: the same device state split into 10-spot zones, to compare
        // with the 3 hot level partitions. Only written when SPOTS_BY_ZONE=on.
        "CREATE TABLE IF NOT EXISTS parkwatch.spots_by_zone (level text, zone text, spot_id text, occupied boolean, plate text, light_reported text, PRIMARY KEY ((level, zone), spot_id))",
        // Phase 7: a counter kept next to the real state, to watch it drift.
        "CREATE TABLE IF NOT EXISTS parkwatch.free_count_by_level (level text PRIMARY KEY, free counter)",
        // Phase 7: monthly passes, checked at the exit gate.
        "CREATE TABLE IF NOT EXISTS parkwatch.passes_by_plate (plate text PRIMARY KEY, holder text, valid_until timestamp)",
    ];

    for statement in DDL {
        session.query_unpaged(statement, ()).await?;
    }
    Ok(())
}

/// The outcome of a lightweight transaction (`... IF ...`).
///
/// Cassandra answers every LWT with a row whose first column is `[applied]`.
/// When the condition failed, the same row also carries the *current* values
/// of the columns involved, which tells us who got there first.
#[derive(Debug)]
pub struct Lwt {
    pub applied: bool,
    current: HashMap<String, CqlValue>,
}

impl Lwt {
    fn text(&self, column: &str) -> Option<&str> {
        match self.current.get(column) {
            Some(CqlValue::Text(value)) | Some(CqlValue::Ascii(value)) => Some(value),
            _ => None,
        }
    }

    fn timestamp_millis(&self, column: &str) -> Option<i64> {
        match self.current.get(column) {
            Some(CqlValue::Timestamp(ts)) => Some(ts.0),
            _ => None,
        }
    }
}

/// Device data: occupancy, lights, heartbeats, history, "find my car".
///
/// `ONE` means a single replica has to acknowledge, so these keep working
/// with 2 of 3 nodes down. The price is that a read may hit a replica that
/// missed the latest write. For a light colour that is fixed by the next
/// event or heartbeat, and hints and repair bring the replica back in sync.
pub fn relaxed(cql: impl Into<String>) -> Statement {
    let mut statement = Statement::new(cql);
    statement.set_consistency(Consistency::One);
    statement
}

/// Money and promises to drivers: sessions, fees, reports, overrides, passes.
///
/// `QUORUM` is 2 of 3 replicas. A quorum write and a quorum read always
/// overlap in at least one replica, so the read sees the write.
pub fn strict(cql: impl Into<String>) -> Statement {
    let mut statement = Statement::new(cql);
    statement.set_consistency(Consistency::Quorum);
    statement
}

/// Runs a lightweight transaction and decodes `[applied]`.
///
/// The Paxos rounds run at `SERIAL` (a majority of all replicas), and the
/// commit is written at `QUORUM`. Both need 2 of 3 nodes, so LWTs stop
/// working when 2 nodes are down. That is the point: the alternative would
/// be two drivers holding the same spot.
///
/// Paxos rounds on one partition compete with each other. Under heavy
/// contention (100 drivers reserving one spot) Cassandra gives up with a
/// `WriteTimeout` of type `CAS`, which means "outcome unknown": the write
/// may or may not have committed. We retry a few times. That is only safe
/// because every caller recognises its own earlier write in the `[applied] =
/// false` answer (same `reserved_by`, same `entered_at`, ...).
pub async fn execute_lwt(db: &Db, cql: &str, values: impl SerializeRow + Clone) -> Result<Lwt> {
    const ATTEMPTS: u64 = 4;

    let mut statement = strict(cql);
    statement.set_serial_consistency(Some(SerialConsistency::Serial));

    for attempt in 1..=ATTEMPTS {
        match db.execute_unpaged(statement.clone(), values.clone()).await {
            Ok(result) => return decode_lwt(result),
            Err(err) if is_cas_timeout(&err) && attempt < ATTEMPTS => {
                tracing::debug!(attempt, %cql, "LWT contention, retrying");
                tokio::time::sleep(Duration::from_millis(20 * attempt)).await;
            }
            Err(err) => return Err(err.into()),
        }
    }
    unreachable!("the loop returns on the last attempt")
}

fn decode_lwt(result: QueryResult) -> Result<Lwt> {
    let rows = result.into_rows_result()?;
    let names: Vec<String> = rows
        .column_specs()
        .iter()
        .map(|spec| spec.name().to_owned())
        .collect();
    let row = rows.first_row::<Row>()?;

    let mut applied = false;
    let mut current = HashMap::new();
    for (name, value) in names.into_iter().zip(row.columns) {
        match (name.as_str(), value) {
            ("[applied]", Some(CqlValue::Boolean(flag))) => applied = flag,
            (_, Some(value)) => {
                current.insert(name, value);
            }
            (_, None) => {}
        }
    }
    Ok(Lwt { applied, current })
}

fn is_cas_timeout(err: &ExecutionError) -> bool {
    matches!(
        err,
        ExecutionError::LastAttemptError(RequestAttemptError::DbError(
            DbError::WriteTimeout {
                write_type: WriteType::Cas,
                ..
            },
            _,
        ))
    )
}

/// Builds a driver session, retrying while the cluster is still starting.
///
/// `contact_points` are only the *first* addresses the driver dials. After it
/// connects, it reads `system.peers` to discover every other node and opens
/// connections to them too. That is why parkd must run inside the compose
/// network: the peers are advertised by their Docker-internal IPs.
pub async fn connect(contact_points: &[String]) -> Result<Session> {
    const ATTEMPTS: u32 = 30;

    for attempt in 1..=ATTEMPTS {
        match SessionBuilder::new()
            .known_nodes(contact_points)
            .connection_timeout(Duration::from_secs(5))
            .build()
            .await
        {
            Ok(session) => return Ok(session),
            Err(err) if attempt < ATTEMPTS => {
                tracing::warn!(attempt, %err, "Cassandra not reachable yet, retrying in 2s");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(err) => {
                return Err(err).with_context(|| {
                    format!("could not connect to Cassandra at {contact_points:?}")
                });
            }
        }
    }
    unreachable!("the loop returns on the last attempt")
}

/// What the driver currently knows about the cluster.
#[derive(Debug, Serialize)]
pub struct ClusterInfo {
    pub cluster_name: String,
    pub release_version: String,
    pub nodes: Vec<NodeInfo>,
}

#[derive(Debug, Serialize)]
pub struct NodeInfo {
    pub address: String,
    pub datacenter: Option<String>,
    pub rack: Option<String>,
    /// True when the driver holds at least one open connection to the node.
    pub connected: bool,
}

pub async fn cluster_info(session: &Session) -> Result<ClusterInfo> {
    // `system.local` describes the node that coordinated this query.
    let (cluster_name, release_version) = session
        .query_unpaged(
            "SELECT cluster_name, release_version FROM system.local",
            &[],
        )
        .await?
        .into_rows_result()?
        .single_row::<(String, String)>()?;

    // The driver's own view of the ring, built from `system.peers` and
    // kept up to date from server push events (node up/down, topology changes).
    let state = session.get_cluster_state();
    let mut nodes: Vec<NodeInfo> = state
        .get_nodes_info()
        .iter()
        .map(|node| NodeInfo {
            address: node.address.to_string(),
            datacenter: node.datacenter.clone(),
            rack: node.rack.clone(),
            connected: node.is_connected(),
        })
        .collect();
    nodes.sort_by(|a, b| a.address.cmp(&b.address));

    Ok(ClusterInfo {
        cluster_name,
        release_version,
        nodes,
    })
}
