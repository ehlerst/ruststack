use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum OrganizationsError {
    #[error("AlreadyInOrganizationException: The account is already part of an organization")]
    AlreadyInOrganization,
    #[error("AWSOrganizationsNotInUseException: Your account is not a member of an organization")]
    NotInUse,
    #[error("AccountNotFoundException: Account {0} not found")]
    AccountNotFound(String),
    #[error("DuplicateAccountException: Account with name {0} already exists")]
    DuplicateAccount(String),
    #[error("InvalidInputException: {0}")]
    InvalidInput(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationsStateSnapshot {
    pub organization: Option<Organization>,
    pub accounts: Vec<Account>,
    pub roots: Vec<Root>,
    pub organizational_units: Vec<(String, OrganizationalUnit)>, // parent_id -> OU
}

#[derive(Clone)]
pub struct OrganizationsState {
    account_id: String,
    region: String,
    organization: Arc<parking_lot::RwLock<Option<Organization>>>,
    accounts: Arc<DashMap<String, Account>>,
    roots: Arc<DashMap<String, Root>>,
    ous: Arc<DashMap<String, Vec<OrganizationalUnit>>>, // parent_id -> list of OUs
}

impl OrganizationsState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            organization: Arc::new(parking_lot::RwLock::new(None)),
            accounts: Arc::new(DashMap::new()),
            roots: Arc::new(DashMap::new()),
            ous: Arc::new(DashMap::new()),
        }
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn create_organization(&self, feature_set: Option<String>) -> Result<Organization, OrganizationsError> {
        let mut org_lock = self.organization.write();
        if org_lock.is_some() {
            return Err(OrganizationsError::AlreadyInOrganization);
        }

        let org_id = format!("o-{}", &Uuid::new_v4().simple().to_string()[0..10]);
        let root_id = format!("r-{}", &Uuid::new_v4().simple().to_string()[0..4]);
        let master_email = "admin@example.com".to_string();

        let org = Organization {
            id: org_id.clone(),
            arn: format!("arn:aws:organizations::{}:organization/{}", self.account_id, org_id),
            feature_set: feature_set.unwrap_or_else(|| "ALL".to_string()),
            master_account_arn: format!("arn:aws:organizations::{}:account/{}/{}", self.account_id, org_id, self.account_id),
            master_account_id: self.account_id.clone(),
            master_account_email: master_email.clone(),
            available_policy_types: vec![],
        };

        let root = Root {
            id: root_id.clone(),
            arn: format!("arn:aws:organizations::{}:root/{}/{}", self.account_id, org_id, root_id),
            name: "Root".to_string(),
            policy_types: vec![],
        };

        let master_account = Account {
            id: self.account_id.clone(),
            arn: org.master_account_arn.clone(),
            email: master_email,
            name: "Master Account".to_string(),
            status: "ACTIVE".to_string(),
            joined_method: "CREATED".to_string(),
            joined_timestamp: Utc::now(),
        };

        *org_lock = Some(org.clone());
        self.roots.insert(root_id, root);
        self.accounts.insert(self.account_id.clone(), master_account);

        Ok(org)
    }

    pub fn describe_organization(&self) -> Result<Organization, OrganizationsError> {
        self.organization
            .read()
            .clone()
            .ok_or(OrganizationsError::NotInUse)
    }

    pub fn create_account(&self, account_name: String, email: String) -> Result<Account, OrganizationsError> {
        let org = self.describe_organization()?;

        for kv in self.accounts.iter() {
            if kv.value().name == account_name || kv.value().email == email {
                return Err(OrganizationsError::DuplicateAccount(account_name));
            }
        }

        let new_id = format!("{:012}", rand_account_id());
        let account = Account {
            id: new_id.clone(),
            arn: format!("arn:aws:organizations::{}:account/{}/{}", self.account_id, org.id, new_id),
            email,
            name: account_name,
            status: "ACTIVE".to_string(),
            joined_method: "CREATED".to_string(),
            joined_timestamp: Utc::now(),
        };

        self.accounts.insert(new_id, account.clone());
        Ok(account)
    }

    pub fn list_accounts(&self) -> Vec<Account> {
        self.accounts.iter().map(|kv| kv.value().clone()).collect()
    }

    pub fn list_roots(&self) -> Vec<Root> {
        self.roots.iter().map(|kv| kv.value().clone()).collect()
    }

    pub fn create_organizational_unit(
        &self,
        parent_id: String,
        name: String,
    ) -> Result<OrganizationalUnit, OrganizationsError> {
        let org = self.describe_organization()?;
        let ou_id = format!("ou-{}-{}", &parent_id[2..parent_id.len().min(6)], &Uuid::new_v4().simple().to_string()[0..8]);
        let ou = OrganizationalUnit {
            id: ou_id.clone(),
            arn: format!("arn:aws:organizations::{}:ou/{}/{}", self.account_id, org.id, ou_id),
            name,
        };

        self.ous.entry(parent_id).or_default().push(ou.clone());
        Ok(ou)
    }

    pub fn list_organizational_units_for_parent(&self, parent_id: &str) -> Vec<OrganizationalUnit> {
        self.ous.get(parent_id).map(|v| v.clone()).unwrap_or_default()
    }

    pub fn reset(&self) {
        *self.organization.write() = None;
        self.accounts.clear();
        self.roots.clear();
        self.ous.clear();
    }

    pub fn export_snapshot(&self) -> OrganizationsStateSnapshot {
        let mut all_ous = Vec::new();
        for kv in self.ous.iter() {
            for ou in kv.value() {
                all_ous.push((kv.key().clone(), ou.clone()));
            }
        }

        OrganizationsStateSnapshot {
            organization: self.organization.read().clone(),
            accounts: self.accounts.iter().map(|kv| kv.value().clone()).collect(),
            roots: self.roots.iter().map(|kv| kv.value().clone()).collect(),
            organizational_units: all_ous,
        }
    }

    pub fn import_snapshot(&self, snapshot: OrganizationsStateSnapshot) {
        *self.organization.write() = snapshot.organization;
        self.accounts.clear();
        self.roots.clear();
        self.ous.clear();

        for acc in snapshot.accounts {
            self.accounts.insert(acc.id.clone(), acc);
        }
        for r in snapshot.roots {
            self.roots.insert(r.id.clone(), r);
        }
        for (pid, ou) in snapshot.organizational_units {
            self.ous.entry(pid).or_default().push(ou);
        }
    }
}

fn rand_account_id() -> u64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    100_000_000_000 + ((nanos % 899_999_999_999) as u64)
}
