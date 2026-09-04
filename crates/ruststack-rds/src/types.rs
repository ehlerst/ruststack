use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub address: String,
    pub port: i32,
    pub hosted_zone_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DBInstance {
    pub db_instance_identifier: String,
    pub db_instance_class: String,
    pub engine: String,
    pub engine_version: String,
    pub db_instance_status: String,
    pub master_username: String,
    pub db_name: Option<String>,
    pub endpoint: Option<Endpoint>,
    pub allocated_storage: i32,
    pub instance_create_time: DateTime<Utc>,
    pub vpc_security_groups: Vec<String>,
    pub db_cluster_identifier: Option<String>,
    pub multi_az: bool,
    pub storage_type: String,
    pub auto_minor_version_upgrade: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DBCluster {
    pub db_cluster_identifier: String,
    pub engine: String,
    pub engine_version: String,
    pub status: String,
    pub master_username: String,
    pub database_name: Option<String>,
    pub endpoint: Option<String>,
    pub reader_endpoint: Option<String>,
    pub port: i32,
    pub cluster_create_time: DateTime<Utc>,
    pub multi_az: bool,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DBSnapshot {
    pub db_snapshot_identifier: String,
    pub db_instance_identifier: String,
    pub snapshot_create_time: DateTime<Utc>,
    pub engine: String,
    pub allocated_storage: i32,
    pub status: String,
    pub master_username: String,
}
