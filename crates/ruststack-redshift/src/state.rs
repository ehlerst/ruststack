use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RedshiftError {
    #[error("ClusterNotFoundFault: Cluster {0} not found")]
    ClusterNotFound(String),
    #[error("ClusterAlreadyExistsFault: Cluster {0} already exists")]
    ClusterAlreadyExists(String),
    #[error("ClusterSnapshotNotFoundFault: Snapshot {0} not found")]
    SnapshotNotFound(String),
    #[error("InvalidParameterValue: {0}")]
    InvalidParameter(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedshiftStateSnapshot {
    pub clusters: Vec<RedshiftCluster>,
    pub snapshots: Vec<ClusterSnapshot>,
}

#[derive(Clone)]
pub struct RedshiftState {
    account_id: String,
    region: String,
    clusters: Arc<DashMap<String, RedshiftCluster>>,
    snapshots: Arc<DashMap<String, ClusterSnapshot>>,
}

impl RedshiftState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            clusters: Arc::new(DashMap::new()),
            snapshots: Arc::new(DashMap::new()),
        }
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn create_cluster(
        &self,
        identifier: String,
        node_type: Option<String>,
        master_username: Option<String>,
        db_name: Option<String>,
        number_of_nodes: Option<i32>,
        encrypted: Option<bool>,
    ) -> Result<RedshiftCluster, RedshiftError> {
        if self.clusters.contains_key(&identifier) {
            return Err(RedshiftError::ClusterAlreadyExists(identifier));
        }

        let num_nodes = number_of_nodes.unwrap_or(1);
        let now = Utc::now();
        let endpoint = RedshiftEndpoint {
            address: format!("{}.{}.redshift.localhost.localstack.cloud", identifier, self.region),
            port: 5439,
        };

        let mut nodes = Vec::new();
        for i in 0..num_nodes {
            nodes.push(ClusterNode {
                node_role: if i == 0 { "LEADER".to_string() } else { "COMPUTE".to_string() },
                private_ip_address: format!("10.0.0.{}", 10 + i),
                public_ip_address: format!("54.214.13.{}", 10 + i),
            });
        }

        let cluster = RedshiftCluster {
            cluster_identifier: identifier.clone(),
            node_type: node_type.unwrap_or_else(|| "dc2.large".to_string()),
            cluster_status: "available".to_string(),
            cluster_availability_status: "Available".to_string(),
            master_username: master_username.unwrap_or_else(|| "admin".to_string()),
            db_name: db_name.unwrap_or_else(|| "dev".to_string()),
            endpoint: Some(endpoint),
            cluster_create_time: now,
            number_of_nodes: num_nodes,
            cluster_nodes: nodes,
            automated_snapshot_retention_period: 1,
            manual_snapshot_retention_period: -1,
            encrypted: encrypted.unwrap_or(false),
            enhanced_vpc_routing: false,
        };

        self.clusters.insert(identifier, cluster.clone());
        Ok(cluster)
    }

    pub fn describe_clusters(&self, identifier: Option<&str>) -> Result<Vec<RedshiftCluster>, RedshiftError> {
        if let Some(id) = identifier {
            let cl = self
                .clusters
                .get(id)
                .map(|kv| kv.value().clone())
                .ok_or_else(|| RedshiftError::ClusterNotFound(id.to_string()))?;
            Ok(vec![cl])
        } else {
            Ok(self.clusters.iter().map(|kv| kv.value().clone()).collect())
        }
    }

    pub fn delete_cluster(&self, identifier: &str) -> Result<RedshiftCluster, RedshiftError> {
        let mut cl = self
            .clusters
            .get_mut(identifier)
            .ok_or_else(|| RedshiftError::ClusterNotFound(identifier.to_string()))?;
        cl.cluster_status = "deleting".to_string();
        let res = cl.clone();
        drop(cl);
        self.clusters.remove(identifier);
        Ok(res)
    }

    pub fn create_cluster_snapshot(
        &self,
        snapshot_id: String,
        cluster_id: String,
    ) -> Result<ClusterSnapshot, RedshiftError> {
        let cl = self
            .clusters
            .get(&cluster_id)
            .ok_or_else(|| RedshiftError::ClusterNotFound(cluster_id.clone()))?;

        let snapshot = ClusterSnapshot {
            snapshot_identifier: snapshot_id.clone(),
            cluster_identifier: cluster_id,
            snapshot_create_time: Utc::now(),
            status: "available".to_string(),
            node_type: cl.node_type.clone(),
            number_of_nodes: cl.number_of_nodes,
            db_name: cl.db_name.clone(),
            master_username: cl.master_username.clone(),
            encrypted: cl.encrypted,
        };

        self.snapshots.insert(snapshot_id, snapshot.clone());
        Ok(snapshot)
    }

    pub fn describe_cluster_snapshots(&self, snapshot_id: Option<&str>) -> Result<Vec<ClusterSnapshot>, RedshiftError> {
        if let Some(id) = snapshot_id {
            let sn = self
                .snapshots
                .get(id)
                .map(|kv| kv.value().clone())
                .ok_or_else(|| RedshiftError::SnapshotNotFound(id.to_string()))?;
            Ok(vec![sn])
        } else {
            Ok(self.snapshots.iter().map(|kv| kv.value().clone()).collect())
        }
    }

    pub fn reset(&self) {
        self.clusters.clear();
        self.snapshots.clear();
    }

    pub fn export_snapshot(&self) -> RedshiftStateSnapshot {
        RedshiftStateSnapshot {
            clusters: self.clusters.iter().map(|kv| kv.value().clone()).collect(),
            snapshots: self.snapshots.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: RedshiftStateSnapshot) {
        self.clusters.clear();
        self.snapshots.clear();
        for cl in snapshot.clusters {
            self.clusters.insert(cl.cluster_identifier.clone(), cl);
        }
        for sn in snapshot.snapshots {
            self.snapshots.insert(sn.snapshot_identifier.clone(), sn);
        }
    }
}
