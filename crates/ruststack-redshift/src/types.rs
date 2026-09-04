use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedshiftEndpoint {
    pub address: String,
    pub port: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNode {
    pub node_role: String,
    pub private_ip_address: String,
    pub public_ip_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedshiftCluster {
    pub cluster_identifier: String,
    pub node_type: String,
    pub cluster_status: String,
    pub cluster_availability_status: String,
    pub master_username: String,
    pub db_name: String,
    pub endpoint: Option<RedshiftEndpoint>,
    pub cluster_create_time: DateTime<Utc>,
    pub number_of_nodes: i32,
    pub cluster_nodes: Vec<ClusterNode>,
    pub automated_snapshot_retention_period: i32,
    pub manual_snapshot_retention_period: i32,
    pub encrypted: bool,
    pub enhanced_vpc_routing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSnapshot {
    pub snapshot_identifier: String,
    pub cluster_identifier: String,
    pub snapshot_create_time: DateTime<Utc>,
    pub status: String,
    pub node_type: String,
    pub number_of_nodes: i32,
    pub db_name: String,
    pub master_username: String,
    pub encrypted: bool,
}
