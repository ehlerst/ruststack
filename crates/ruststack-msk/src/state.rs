use crate::types::{BootstrapBrokers, Cluster, ClusterSummary, MskError};
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MskSnapshot {
    pub clusters: Vec<Cluster>,
}

#[derive(Clone)]
pub struct MskState {
    pub account_id: String,
    pub region: String,
    clusters: Arc<DashMap<String, Cluster>>,
}

impl MskState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            clusters: Arc::new(DashMap::new()),
        }
    }

    pub fn export_snapshot(&self) -> MskSnapshot {
        MskSnapshot {
            clusters: self.clusters.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: MskSnapshot) {
        self.clusters.clear();
        for cluster in snapshot.clusters {
            self.clusters.insert(cluster.cluster_arn.clone(), cluster);
        }
    }

    pub fn reset(&self) {
        self.clusters.clear();
    }

    pub fn create_cluster(
        &self,
        cluster_name: String,
        number_of_broker_nodes: i32,
        kafka_version: Option<String>,
        tags: Option<HashMap<String, String>>,
    ) -> Result<Cluster, MskError> {
        for kv in self.clusters.iter() {
            if kv.value().cluster_name == cluster_name {
                return Err(MskError::ClusterAlreadyExists(cluster_name));
            }
        }

        let cluster_uuid = Uuid::new_v4().to_string();
        let cluster_arn = format!(
            "arn:aws:kafka:{}:{}:cluster/{}/{}",
            self.region, self.account_id, cluster_name, cluster_uuid
        );
        let now = Utc::now().to_rfc3339();
        let k_ver = kafka_version.unwrap_or_else(|| "3.5.1".to_string());
        let zk_conn = format!(
            "z-1.{}.{}.kafka.{}.amazonaws.com:2181,z-2.{}.{}.kafka.{}.amazonaws.com:2181",
            cluster_name, cluster_uuid, self.region, cluster_name, cluster_uuid, self.region
        );

        let cluster = Cluster {
            cluster_arn: cluster_arn.clone(),
            cluster_name,
            creation_time: now,
            current_version: "K3AEGXETSR30GU".to_string(),
            state: "ACTIVE".to_string(),
            number_of_broker_nodes,
            kafka_version: k_ver,
            zookeeper_connect_string: zk_conn,
            tags: tags.unwrap_or_default(),
        };

        self.clusters.insert(cluster_arn, cluster.clone());
        Ok(cluster)
    }

    pub fn describe_cluster(&self, identifier: &str) -> Result<Cluster, MskError> {
        if let Some(c) = self.clusters.get(identifier) {
            return Ok(c.clone());
        }
        for kv in self.clusters.iter() {
            if kv.value().cluster_name == identifier {
                return Ok(kv.value().clone());
            }
        }
        Err(MskError::ClusterNotFound(identifier.to_string()))
    }

    pub fn list_clusters(&self) -> Vec<ClusterSummary> {
        let mut list: Vec<ClusterSummary> = self
            .clusters
            .iter()
            .map(|kv| {
                let c = kv.value();
                ClusterSummary {
                    cluster_arn: c.cluster_arn.clone(),
                    cluster_name: c.cluster_name.clone(),
                    creation_time: c.creation_time.clone(),
                    current_version: c.current_version.clone(),
                    state: c.state.clone(),
                    number_of_broker_nodes: c.number_of_broker_nodes,
                    kafka_version: c.kafka_version.clone(),
                }
            })
            .collect();

        list.sort_by_key(|a| a.cluster_name.clone());
        list
    }

    pub fn delete_cluster(&self, identifier: &str) -> Result<Cluster, MskError> {
        let key = if self.clusters.contains_key(identifier) {
            Some(identifier.to_string())
        } else {
            self.clusters
                .iter()
                .find(|kv| kv.value().cluster_name == identifier)
                .map(|kv| kv.key().clone())
        };

        match key {
            Some(k) => self
                .clusters
                .remove(&k)
                .map(|(_, c)| c)
                .ok_or_else(|| MskError::ClusterNotFound(identifier.to_string())),
            None => Err(MskError::ClusterNotFound(identifier.to_string())),
        }
    }

    pub fn get_bootstrap_brokers(&self, identifier: &str) -> Result<BootstrapBrokers, MskError> {
        let cluster = self.describe_cluster(identifier)?;
        let name = &cluster.cluster_name;
        let mut brokers = Vec::new();
        let mut brokers_tls = Vec::new();
        let mut brokers_sasl = Vec::new();

        for i in 1..=cluster.number_of_broker_nodes {
            brokers.push(format!("b-{}.{}.kafka.{}:9092", i, name, self.region));
            brokers_tls.push(format!("b-{}.{}.kafka.{}:9094", i, name, self.region));
            brokers_sasl.push(format!("b-{}.{}.kafka.{}:9096", i, name, self.region));
        }

        Ok(BootstrapBrokers {
            bootstrap_broker_string: Some(brokers.join(",")),
            bootstrap_broker_string_tls: Some(brokers_tls.join(",")),
            bootstrap_broker_string_sasl_scram: Some(brokers_sasl.join(",")),
        })
    }
}
