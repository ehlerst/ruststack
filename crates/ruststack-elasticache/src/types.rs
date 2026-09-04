use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEndpoint {
    pub address: String,
    pub port: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheNode {
    pub cache_node_id: String,
    pub cache_node_status: String,
    pub cache_node_create_time: DateTime<Utc>,
    pub endpoint: Option<CacheEndpoint>,
    pub customer_availability_zone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheCluster {
    pub cache_cluster_identifier: String,
    pub configuration_endpoint: Option<CacheEndpoint>,
    pub client_download_landing_page: Option<String>,
    pub cache_node_type: String,
    pub engine: String,
    pub engine_version: String,
    pub cache_cluster_status: String,
    pub num_cache_nodes: i32,
    pub preferred_availability_zone: String,
    pub cache_cluster_create_time: DateTime<Utc>,
    pub cache_nodes: Vec<CacheNode>,
    pub replication_group_id: Option<String>,
    pub auto_minor_version_upgrade: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGroupMember {
    pub cache_cluster_id: String,
    pub cache_node_id: String,
    pub read_endpoint: Option<CacheEndpoint>,
    pub preferred_availability_zone: String,
    pub current_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGroup {
    pub node_group_id: String,
    pub status: String,
    pub primary_endpoint: Option<CacheEndpoint>,
    pub reader_endpoint: Option<CacheEndpoint>,
    pub node_group_members: Vec<NodeGroupMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationGroup {
    pub replication_group_id: String,
    pub description: String,
    pub status: String,
    pub member_clusters: Vec<String>,
    pub node_groups: Vec<NodeGroup>,
    pub primary_endpoint: Option<CacheEndpoint>,
    pub reader_endpoint: Option<CacheEndpoint>,
    pub multi_az: String,
    pub automatic_failover: String,
}
