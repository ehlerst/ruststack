use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ElastiCacheError {
    #[error("CacheClusterNotFound: CacheCluster {0} not found")]
    CacheClusterNotFound(String),
    #[error("CacheClusterAlreadyExists: CacheCluster {0} already exists")]
    CacheClusterAlreadyExists(String),
    #[error("ReplicationGroupNotFoundFault: ReplicationGroup {0} not found")]
    ReplicationGroupNotFound(String),
    #[error("ReplicationGroupAlreadyExistsFault: ReplicationGroup {0} already exists")]
    ReplicationGroupAlreadyExists(String),
    #[error("InvalidParameterValue: {0}")]
    InvalidParameter(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElastiCacheStateSnapshot {
    pub clusters: Vec<CacheCluster>,
    pub replication_groups: Vec<ReplicationGroup>,
}

#[derive(Clone)]
pub struct ElastiCacheState {
    account_id: String,
    region: String,
    clusters: Arc<DashMap<String, CacheCluster>>,
    replication_groups: Arc<DashMap<String, ReplicationGroup>>,
}

impl ElastiCacheState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            clusters: Arc::new(DashMap::new()),
            replication_groups: Arc::new(DashMap::new()),
        }
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn create_cache_cluster(
        &self,
        identifier: String,
        cache_node_type: Option<String>,
        engine: Option<String>,
        engine_version: Option<String>,
        num_cache_nodes: Option<i32>,
        replication_group_id: Option<String>,
    ) -> Result<CacheCluster, ElastiCacheError> {
        if self.clusters.contains_key(&identifier) {
            return Err(ElastiCacheError::CacheClusterAlreadyExists(identifier));
        }

        let engine = engine.unwrap_or_else(|| "redis".to_string());
        let port = if engine == "redis" { 6379 } else { 11211 };
        let num_nodes = num_cache_nodes.unwrap_or(1);
        let now = Utc::now();

        let endpoint = CacheEndpoint {
            address: format!("{}.{}.cache.localhost.localstack.cloud", identifier, self.region),
            port,
        };

        let mut nodes = Vec::new();
        for i in 1..=num_nodes {
            let node_id = format!("{:04}", i);
            nodes.push(CacheNode {
                cache_node_id: node_id,
                cache_node_status: "available".to_string(),
                cache_node_create_time: now,
                endpoint: Some(endpoint.clone()),
                customer_availability_zone: format!("{}a", self.region),
            });
        }

        let cluster = CacheCluster {
            cache_cluster_identifier: identifier.clone(),
            configuration_endpoint: if engine == "memcached" { Some(endpoint.clone()) } else { None },
            client_download_landing_page: None,
            cache_node_type: cache_node_type.unwrap_or_else(|| "cache.t3.micro".to_string()),
            engine: engine.clone(),
            engine_version: engine_version.unwrap_or_else(|| "7.0".to_string()),
            cache_cluster_status: "available".to_string(),
            num_cache_nodes: num_nodes,
            preferred_availability_zone: format!("{}a", self.region),
            cache_cluster_create_time: now,
            cache_nodes: nodes,
            replication_group_id,
            auto_minor_version_upgrade: true,
        };

        self.clusters.insert(identifier, cluster.clone());
        Ok(cluster)
    }

    pub fn describe_cache_clusters(&self, identifier: Option<&str>) -> Result<Vec<CacheCluster>, ElastiCacheError> {
        if let Some(id) = identifier {
            let cl = self
                .clusters
                .get(id)
                .map(|kv| kv.value().clone())
                .ok_or_else(|| ElastiCacheError::CacheClusterNotFound(id.to_string()))?;
            Ok(vec![cl])
        } else {
            Ok(self.clusters.iter().map(|kv| kv.value().clone()).collect())
        }
    }

    pub fn delete_cache_cluster(&self, identifier: &str) -> Result<CacheCluster, ElastiCacheError> {
        let mut cl = self
            .clusters
            .get_mut(identifier)
            .ok_or_else(|| ElastiCacheError::CacheClusterNotFound(identifier.to_string()))?;
        cl.cache_cluster_status = "deleting".to_string();
        let res = cl.clone();
        drop(cl);
        self.clusters.remove(identifier);
        Ok(res)
    }

    pub fn create_replication_group(
        &self,
        identifier: String,
        description: String,
        num_cache_clusters: Option<i32>,
    ) -> Result<ReplicationGroup, ElastiCacheError> {
        if self.replication_groups.contains_key(&identifier) {
            return Err(ElastiCacheError::ReplicationGroupAlreadyExists(identifier));
        }

        let endpoint = CacheEndpoint {
            address: format!("{}.{}.cache.localhost.localstack.cloud", identifier, self.region),
            port: 6379,
        };

        let num_clusters = num_cache_clusters.unwrap_or(2);
        let mut member_clusters = Vec::new();
        let mut members = Vec::new();

        for i in 1..=num_clusters {
            let cluster_id = format!("{}-{:03}", identifier, i);
            member_clusters.push(cluster_id.clone());
            members.push(NodeGroupMember {
                cache_cluster_id: cluster_id,
                cache_node_id: "0001".to_string(),
                read_endpoint: Some(endpoint.clone()),
                preferred_availability_zone: format!("{}a", self.region),
                current_role: if i == 1 { "primary".to_string() } else { "replica".to_string() },
            });
        }

        let node_group = NodeGroup {
            node_group_id: "0001".to_string(),
            status: "available".to_string(),
            primary_endpoint: Some(endpoint.clone()),
            reader_endpoint: Some(endpoint.clone()),
            node_group_members: members,
        };

        let rg = ReplicationGroup {
            replication_group_id: identifier.clone(),
            description,
            status: "available".to_string(),
            member_clusters,
            node_groups: vec![node_group],
            primary_endpoint: Some(endpoint.clone()),
            reader_endpoint: Some(endpoint),
            multi_az: "enabled".to_string(),
            automatic_failover: "enabled".to_string(),
        };

        self.replication_groups.insert(identifier, rg.clone());
        Ok(rg)
    }

    pub fn describe_replication_groups(&self, identifier: Option<&str>) -> Result<Vec<ReplicationGroup>, ElastiCacheError> {
        if let Some(id) = identifier {
            let rg = self
                .replication_groups
                .get(id)
                .map(|kv| kv.value().clone())
                .ok_or_else(|| ElastiCacheError::ReplicationGroupNotFound(id.to_string()))?;
            Ok(vec![rg])
        } else {
            Ok(self.replication_groups.iter().map(|kv| kv.value().clone()).collect())
        }
    }

    pub fn reset(&self) {
        self.clusters.clear();
        self.replication_groups.clear();
    }

    pub fn export_snapshot(&self) -> ElastiCacheStateSnapshot {
        ElastiCacheStateSnapshot {
            clusters: self.clusters.iter().map(|kv| kv.value().clone()).collect(),
            replication_groups: self.replication_groups.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: ElastiCacheStateSnapshot) {
        self.clusters.clear();
        self.replication_groups.clear();
        for cl in snapshot.clusters {
            self.clusters.insert(cl.cache_cluster_identifier.clone(), cl);
        }
        for rg in snapshot.replication_groups {
            self.replication_groups.insert(rg.replication_group_id.clone(), rg);
        }
    }
}
