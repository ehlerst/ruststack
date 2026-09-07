use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    #[serde(rename = "ClusterArn")]
    pub cluster_arn: String,
    #[serde(rename = "ClusterName")]
    pub cluster_name: String,
    #[serde(rename = "CreationTime")]
    pub creation_time: String,
    #[serde(rename = "CurrentVersion")]
    pub current_version: String,
    #[serde(rename = "State")]
    pub state: String,
    #[serde(rename = "NumberOfBrokerNodes")]
    pub number_of_broker_nodes: i32,
    #[serde(rename = "KafkaVersion")]
    pub kafka_version: String,
    #[serde(rename = "ZookeeperConnectString")]
    pub zookeeper_connect_string: String,
    #[serde(rename = "Tags", default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSummary {
    #[serde(rename = "ClusterArn")]
    pub cluster_arn: String,
    #[serde(rename = "ClusterName")]
    pub cluster_name: String,
    #[serde(rename = "CreationTime")]
    pub creation_time: String,
    #[serde(rename = "CurrentVersion")]
    pub current_version: String,
    #[serde(rename = "State")]
    pub state: String,
    #[serde(rename = "NumberOfBrokerNodes")]
    pub number_of_broker_nodes: i32,
    #[serde(rename = "KafkaVersion")]
    pub kafka_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapBrokers {
    #[serde(
        rename = "BootstrapBrokerString",
        skip_serializing_if = "Option::is_none"
    )]
    pub bootstrap_broker_string: Option<String>,
    #[serde(
        rename = "BootstrapBrokerStringTls",
        skip_serializing_if = "Option::is_none"
    )]
    pub bootstrap_broker_string_tls: Option<String>,
    #[serde(
        rename = "BootstrapBrokerStringSaslScram",
        skip_serializing_if = "Option::is_none"
    )]
    pub bootstrap_broker_string_sasl_scram: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum MskError {
    #[error("Cluster with name '{0}' already exists")]
    ClusterAlreadyExists(String),
    #[error("Cluster '{0}' not found")]
    ClusterNotFound(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}
