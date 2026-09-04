use crate::types::*;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum Wafv2Error {
    #[error("WAFNonexistentItemException: The referenced item does not exist: {0}")]
    NonexistentItem(String),
    #[error("WAFDuplicateItemException: The item already exists: {0}")]
    DuplicateItem(String),
    #[error("WAFInvalidParameterException: {0}")]
    InvalidParameter(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wafv2StateSnapshot {
    pub web_acls: Vec<WebACL>,
    pub ip_sets: Vec<IPSet>,
    pub associations: Vec<(String, String)>,
}

#[derive(Clone)]
pub struct Wafv2State {
    account_id: String,
    region: String,
    web_acls: Arc<DashMap<String, WebACL>>,
    ip_sets: Arc<DashMap<String, IPSet>>,
    associations: Arc<DashMap<String, String>>, // resource_arn -> web_acl_arn
}

impl Wafv2State {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            web_acls: Arc::new(DashMap::new()),
            ip_sets: Arc::new(DashMap::new()),
            associations: Arc::new(DashMap::new()),
        }
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn create_web_acl(
        &self,
        name: String,
        scope: &str,
        default_action: serde_json::Value,
        description: Option<String>,
        rules: Option<Vec<serde_json::Value>>,
        visibility_config: VisibilityConfig,
    ) -> Result<WebACLSummary, Wafv2Error> {
        let region = if scope == "CLOUDFRONT" { "global" } else { &self.region };
        let arn_prefix = format!("arn:aws:wafv2:{}:{}:{}/webacl/{}", region, self.account_id, scope.to_lowercase(), name);

        for kv in self.web_acls.iter() {
            if kv.value().name == name {
                return Err(Wafv2Error::DuplicateItem(name));
            }
        }

        let id = Uuid::new_v4().to_string();
        let arn = format!("{}/{}", arn_prefix, id);
        let lock_token = Uuid::new_v4().to_string();

        let acl = WebACL {
            name: name.clone(),
            id: id.clone(),
            arn: arn.clone(),
            default_action,
            description: description.clone(),
            rules: rules.unwrap_or_default(),
            visibility_config,
            capacity: 100,
            lock_token: lock_token.clone(),
        };

        self.web_acls.insert(id.clone(), acl);

        Ok(WebACLSummary {
            name,
            id,
            description,
            lock_token,
            arn,
        })
    }

    pub fn get_web_acl(&self, name: &str, id: &str) -> Result<WebACL, Wafv2Error> {
        let acl = self
            .web_acls
            .get(id)
            .map(|kv| kv.value().clone())
            .ok_or_else(|| Wafv2Error::NonexistentItem(id.to_string()))?;

        if acl.name != name {
            return Err(Wafv2Error::NonexistentItem(name.to_string()));
        }

        Ok(acl)
    }

    pub fn list_web_acls(&self) -> Vec<WebACLSummary> {
        self.web_acls
            .iter()
            .map(|kv| {
                let a = kv.value();
                WebACLSummary {
                    name: a.name.clone(),
                    id: a.id.clone(),
                    description: a.description.clone(),
                    lock_token: a.lock_token.clone(),
                    arn: a.arn.clone(),
                }
            })
            .collect()
    }

    pub fn delete_web_acl(&self, id: &str) -> Result<(), Wafv2Error> {
        self.web_acls
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| Wafv2Error::NonexistentItem(id.to_string()))
    }

    pub fn associate_web_acl(&self, web_acl_arn: String, resource_arn: String) {
        self.associations.insert(resource_arn, web_acl_arn);
    }

    pub fn disassociate_web_acl(&self, resource_arn: &str) {
        self.associations.remove(resource_arn);
    }

    pub fn get_web_acl_for_resource(&self, resource_arn: &str) -> Option<WebACL> {
        let web_acl_arn = self.associations.get(resource_arn)?;
        for kv in self.web_acls.iter() {
            if &kv.value().arn == web_acl_arn.value() {
                return Some(kv.value().clone());
            }
        }
        None
    }

    pub fn create_ip_set(
        &self,
        name: String,
        scope: &str,
        description: Option<String>,
        ip_address_version: String,
        addresses: Vec<String>,
    ) -> Result<IPSetSummary, Wafv2Error> {
        let id = Uuid::new_v4().to_string();
        let region = if scope == "CLOUDFRONT" { "global" } else { &self.region };
        let arn = format!(
            "arn:aws:wafv2:{}:{}:{}/ipset/{}/{}",
            region, self.account_id, scope.to_lowercase(), name, id
        );
        let lock_token = Uuid::new_v4().to_string();

        let ip_set = IPSet {
            name: name.clone(),
            id: id.clone(),
            arn: arn.clone(),
            description: description.clone(),
            ip_address_version,
            addresses,
            lock_token: lock_token.clone(),
        };

        self.ip_sets.insert(id.clone(), ip_set);

        Ok(IPSetSummary {
            name,
            id,
            description,
            lock_token,
            arn,
        })
    }

    pub fn get_ip_set(&self, name: &str, id: &str) -> Result<IPSet, Wafv2Error> {
        let set = self
            .ip_sets
            .get(id)
            .map(|kv| kv.value().clone())
            .ok_or_else(|| Wafv2Error::NonexistentItem(id.to_string()))?;

        if set.name != name {
            return Err(Wafv2Error::NonexistentItem(name.to_string()));
        }

        Ok(set)
    }

    pub fn list_ip_sets(&self) -> Vec<IPSetSummary> {
        self.ip_sets
            .iter()
            .map(|kv| {
                let s = kv.value();
                IPSetSummary {
                    name: s.name.clone(),
                    id: s.id.clone(),
                    description: s.description.clone(),
                    lock_token: s.lock_token.clone(),
                    arn: s.arn.clone(),
                }
            })
            .collect()
    }

    pub fn reset(&self) {
        self.web_acls.clear();
        self.ip_sets.clear();
        self.associations.clear();
    }

    pub fn export_snapshot(&self) -> Wafv2StateSnapshot {
        Wafv2StateSnapshot {
            web_acls: self.web_acls.iter().map(|kv| kv.value().clone()).collect(),
            ip_sets: self.ip_sets.iter().map(|kv| kv.value().clone()).collect(),
            associations: self
                .associations
                .iter()
                .map(|kv| (kv.key().clone(), kv.value().clone()))
                .collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: Wafv2StateSnapshot) {
        self.web_acls.clear();
        self.ip_sets.clear();
        self.associations.clear();
        for a in snapshot.web_acls {
            self.web_acls.insert(a.id.clone(), a);
        }
        for s in snapshot.ip_sets {
            self.ip_sets.insert(s.id.clone(), s);
        }
        for (r, w) in snapshot.associations {
            self.associations.insert(r, w);
        }
    }
}
