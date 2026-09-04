use crate::types::*;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpenSearchError {
    #[error("ResourceNotFoundException: Domain {0} not found")]
    DomainNotFound(String),
    #[error("ResourceAlreadyExistsException: Domain {0} already exists")]
    DomainAlreadyExists(String),
    #[error("ValidationException: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSearchStateSnapshot {
    pub domains: Vec<DomainStatus>,
}

#[derive(Clone)]
pub struct OpenSearchState {
    account_id: String,
    region: String,
    domains: Arc<DashMap<String, DomainStatus>>,
}

impl OpenSearchState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            domains: Arc::new(DashMap::new()),
        }
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn create_domain(&self, req: CreateDomainRequest) -> Result<DomainStatus, OpenSearchError> {
        let name = req.domain_name.clone();
        if self.domains.contains_key(&name) {
            return Err(OpenSearchError::DomainAlreadyExists(name));
        }

        let arn = format!("arn:aws:es:{}:{}:domain/{}", self.region, self.account_id, name);
        let endpoint = format!("{}.{}.es.localhost.localstack.cloud:4566", name, self.region);
        let status = DomainStatus {
            domain_id: format!("{}/{}", self.account_id, name),
            domain_name: name.clone(),
            arn,
            created: true,
            deleted: false,
            endpoint: Some(endpoint),
            engine_version: req.engine_version.unwrap_or_else(|| "OpenSearch_2.11".to_string()),
            cluster_config: req.cluster_config,
            ebs_options: req.ebs_options,
            node_to_node_encryption_options: req.node_to_node_encryption_options,
            processing: false,
            upgrade_processing: false,
        };

        self.domains.insert(name, status.clone());
        Ok(status)
    }

    pub fn describe_domain(&self, domain_name: &str) -> Result<DomainStatus, OpenSearchError> {
        self.domains
            .get(domain_name)
            .map(|kv| kv.value().clone())
            .ok_or_else(|| OpenSearchError::DomainNotFound(domain_name.to_string()))
    }

    pub fn list_domain_names(&self) -> Vec<DomainInfo> {
        self.domains
            .iter()
            .map(|kv| DomainInfo {
                domain_name: kv.key().clone(),
                engine_type: "OpenSearch".to_string(),
            })
            .collect()
    }

    pub fn delete_domain(&self, domain_name: &str) -> Result<DomainStatus, OpenSearchError> {
        let mut status = self
            .domains
            .get_mut(domain_name)
            .ok_or_else(|| OpenSearchError::DomainNotFound(domain_name.to_string()))?;
        status.deleted = true;
        status.processing = false;
        let final_status = status.clone();
        drop(status);
        self.domains.remove(domain_name);
        Ok(final_status)
    }

    pub fn reset(&self) {
        self.domains.clear();
    }

    pub fn export_snapshot(&self) -> OpenSearchStateSnapshot {
        OpenSearchStateSnapshot {
            domains: self.domains.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: OpenSearchStateSnapshot) {
        self.domains.clear();
        for d in snapshot.domains {
            self.domains.insert(d.domain_name.clone(), d);
        }
    }
}
