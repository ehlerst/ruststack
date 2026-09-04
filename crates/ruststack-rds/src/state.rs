use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RdsError {
    #[error("DBInstanceNotFound: DBInstance {0} not found")]
    DBInstanceNotFound(String),
    #[error("DBInstanceAlreadyExists: DBInstance {0} already exists")]
    DBInstanceAlreadyExists(String),
    #[error("DBClusterNotFoundFault: DBCluster {0} not found")]
    DBClusterNotFound(String),
    #[error("DBClusterAlreadyExistsFault: DBCluster {0} already exists")]
    DBClusterAlreadyExists(String),
    #[error("DBSnapshotNotFound: DBSnapshot {0} not found")]
    DBSnapshotNotFound(String),
    #[error("InvalidParameterValue: {0}")]
    InvalidParameter(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdsStateSnapshot {
    pub instances: Vec<DBInstance>,
    pub clusters: Vec<DBCluster>,
    pub snapshots: Vec<DBSnapshot>,
}

#[derive(Clone)]
pub struct RdsState {
    account_id: String,
    region: String,
    instances: Arc<DashMap<String, DBInstance>>,
    clusters: Arc<DashMap<String, DBCluster>>,
    snapshots: Arc<DashMap<String, DBSnapshot>>,
}

impl RdsState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            instances: Arc::new(DashMap::new()),
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

    pub fn create_db_instance(
        &self,
        identifier: String,
        db_instance_class: String,
        engine: String,
        engine_version: Option<String>,
        master_username: String,
        db_name: Option<String>,
        allocated_storage: Option<i32>,
        db_cluster_identifier: Option<String>,
    ) -> Result<DBInstance, RdsError> {
        if self.instances.contains_key(&identifier) {
            return Err(RdsError::DBInstanceAlreadyExists(identifier));
        }

        let port = match engine.to_lowercase().as_str() {
            "postgres" | "aurora-postgresql" => 5432,
            "mysql" | "aurora-mysql" | "mariadb" => 3306,
            "oracle" => 1521,
            "sqlserver" => 1433,
            _ => 5432,
        };

        let endpoint = Endpoint {
            address: format!("{}.{}.rds.localhost.localstack.cloud", identifier, self.region),
            port,
            hosted_zone_id: "Z2YGUSBFPMUBSH".to_string(),
        };

        let instance = DBInstance {
            db_instance_identifier: identifier.clone(),
            db_instance_class,
            engine: engine.clone(),
            engine_version: engine_version.unwrap_or_else(|| "15.4".to_string()),
            db_instance_status: "available".to_string(),
            master_username,
            db_name,
            endpoint: Some(endpoint),
            allocated_storage: allocated_storage.unwrap_or(20),
            instance_create_time: Utc::now(),
            vpc_security_groups: vec!["sg-default".to_string()],
            db_cluster_identifier,
            multi_az: false,
            storage_type: "gp2".to_string(),
            auto_minor_version_upgrade: true,
        };

        self.instances.insert(identifier, instance.clone());
        Ok(instance)
    }

    pub fn describe_db_instances(&self, identifier: Option<&str>) -> Result<Vec<DBInstance>, RdsError> {
        if let Some(id) = identifier {
            let inst = self
                .instances
                .get(id)
                .map(|kv| kv.value().clone())
                .ok_or_else(|| RdsError::DBInstanceNotFound(id.to_string()))?;
            Ok(vec![inst])
        } else {
            Ok(self.instances.iter().map(|kv| kv.value().clone()).collect())
        }
    }

    pub fn delete_db_instance(&self, identifier: &str) -> Result<DBInstance, RdsError> {
        let mut inst = self
            .instances
            .get_mut(identifier)
            .ok_or_else(|| RdsError::DBInstanceNotFound(identifier.to_string()))?;
        inst.db_instance_status = "deleting".to_string();
        let res = inst.clone();
        drop(inst);
        self.instances.remove(identifier);
        Ok(res)
    }

    pub fn create_db_cluster(
        &self,
        identifier: String,
        engine: String,
        engine_version: Option<String>,
        master_username: String,
        database_name: Option<String>,
    ) -> Result<DBCluster, RdsError> {
        if self.clusters.contains_key(&identifier) {
            return Err(RdsError::DBClusterAlreadyExists(identifier));
        }

        let port = match engine.to_lowercase().as_str() {
            "postgres" | "aurora-postgresql" => 5432,
            _ => 3306,
        };

        let cluster = DBCluster {
            db_cluster_identifier: identifier.clone(),
            engine: engine.clone(),
            engine_version: engine_version.unwrap_or_else(|| "15.4".to_string()),
            status: "available".to_string(),
            master_username,
            database_name,
            endpoint: Some(format!("{}.cluster-{}.rds.localhost.localstack.cloud", identifier, self.region)),
            reader_endpoint: Some(format!("{}.cluster-ro-{}.rds.localhost.localstack.cloud", identifier, self.region)),
            port,
            cluster_create_time: Utc::now(),
            multi_az: true,
            members: vec![],
        };

        self.clusters.insert(identifier, cluster.clone());
        Ok(cluster)
    }

    pub fn describe_db_clusters(&self, identifier: Option<&str>) -> Result<Vec<DBCluster>, RdsError> {
        if let Some(id) = identifier {
            let cl = self
                .clusters
                .get(id)
                .map(|kv| kv.value().clone())
                .ok_or_else(|| RdsError::DBClusterNotFound(id.to_string()))?;
            Ok(vec![cl])
        } else {
            Ok(self.clusters.iter().map(|kv| kv.value().clone()).collect())
        }
    }

    pub fn create_db_snapshot(
        &self,
        snapshot_id: String,
        instance_id: String,
    ) -> Result<DBSnapshot, RdsError> {
        let inst = self
            .instances
            .get(&instance_id)
            .ok_or_else(|| RdsError::DBInstanceNotFound(instance_id.clone()))?;

        let snapshot = DBSnapshot {
            db_snapshot_identifier: snapshot_id.clone(),
            db_instance_identifier: instance_id,
            snapshot_create_time: Utc::now(),
            engine: inst.engine.clone(),
            allocated_storage: inst.allocated_storage,
            status: "available".to_string(),
            master_username: inst.master_username.clone(),
        };

        self.snapshots.insert(snapshot_id, snapshot.clone());
        Ok(snapshot)
    }

    pub fn reset(&self) {
        self.instances.clear();
        self.clusters.clear();
        self.snapshots.clear();
    }

    pub fn export_snapshot(&self) -> RdsStateSnapshot {
        RdsStateSnapshot {
            instances: self.instances.iter().map(|kv| kv.value().clone()).collect(),
            clusters: self.clusters.iter().map(|kv| kv.value().clone()).collect(),
            snapshots: self.snapshots.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: RdsStateSnapshot) {
        self.instances.clear();
        self.clusters.clear();
        self.snapshots.clear();
        for inst in snapshot.instances {
            self.instances.insert(inst.db_instance_identifier.clone(), inst);
        }
        for cl in snapshot.clusters {
            self.clusters.insert(cl.db_cluster_identifier.clone(), cl);
        }
        for sn in snapshot.snapshots {
            self.snapshots.insert(sn.db_snapshot_identifier.clone(), sn);
        }
    }
}
