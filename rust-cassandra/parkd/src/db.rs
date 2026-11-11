//! Connecting to the Cassandra cluster.

use std::time::Duration;

use anyhow::{Context, Result};
use scylla::client::session::Session;
use scylla::client::session_builder::SessionBuilder;
use serde::Serialize;

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
