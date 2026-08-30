use crate::types::{CreateSecretRequest, Secret, SecretVersion};
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use ruststack_core::RustStackError;
use std::collections::HashMap;
use std::sync::Arc;

pub struct SecretsManagerEngine {
    secrets_by_name: DashMap<String, Arc<RwLock<Secret>>>,
    secrets_by_arn: DashMap<String, Arc<RwLock<Secret>>>,
    account_id: String,
    region: String,
}

impl SecretsManagerEngine {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            secrets_by_name: DashMap::new(),
            secrets_by_arn: DashMap::new(),
            account_id,
            region,
        }
    }

    pub fn format_secret_arn(&self, name: &str) -> String {
        format!(
            "arn:aws:secretsmanager:{}:{}:secret:{}-{}",
            self.region,
            self.account_id,
            name,
            &uuid::Uuid::new_v4().to_string()[..6]
        )
    }

    fn find_secret(&self, secret_id: &str) -> Option<Arc<RwLock<Secret>>> {
        if let Some(s) = self.secrets_by_arn.get(secret_id) {
            return Some(s.clone());
        }
        if let Some(s) = self.secrets_by_name.get(secret_id) {
            return Some(s.clone());
        }
        None
    }

    pub fn create_secret(
        &self,
        req: CreateSecretRequest,
    ) -> Result<(String, String), RustStackError> {
        let name = req.name.clone();
        if self.secrets_by_name.contains_key(&name) {
            return Err(RustStackError::secretsmanager_bad_request(
                "ResourceExistsException",
                format!("The secret {} already exists.", name),
            ));
        }

        let arn = self.format_secret_arn(&name);
        let now = Utc::now();
        let version_id = req
            .client_request_token
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let mut versions = HashMap::new();
        if req.secret_string.is_some() || req.secret_binary.is_some() {
            let ver = SecretVersion {
                version_id: version_id.clone(),
                secret_string: req.secret_string,
                secret_binary: req.secret_binary,
                version_stages: vec!["AWSCURRENT".to_string()],
                created_date: now,
            };
            versions.insert(version_id.clone(), ver);
        }

        let current_ver = if versions.is_empty() {
            None
        } else {
            Some(version_id.clone())
        };

        let secret = Secret {
            arn: arn.clone(),
            name: name.clone(),
            description: req.description,
            kms_key_id: req.kms_key_id,
            deleted_date: None,
            created_date: now,
            last_accessed_date: None,
            last_changed_date: Some(now),
            versions,
            current_version_id: current_ver,
        };

        let secret_arc = Arc::new(RwLock::new(secret));
        self.secrets_by_name.insert(name, secret_arc.clone());
        self.secrets_by_arn.insert(arn.clone(), secret_arc);

        Ok((arn, version_id))
    }

    pub fn get_secret_value(
        &self,
        secret_id: &str,
        version_id_opt: Option<&str>,
        version_stage_opt: Option<&str>,
    ) -> Result<SecretVersion, RustStackError> {
        let secret_arc = self.find_secret(secret_id).ok_or_else(|| {
            RustStackError::secretsmanager_not_found(
                "ResourceNotFoundException",
                format!("Secret {} not found.", secret_id),
            )
        })?;

        let mut secret = secret_arc.write();
        if secret.deleted_date.is_some() {
            return Err(RustStackError::secretsmanager_bad_request(
                "InvalidRequestException",
                format!("Secret {} is marked for deletion.", secret_id),
            ));
        }

        secret.last_accessed_date = Some(Utc::now());

        if let Some(target_vid) = version_id_opt {
            if let Some(ver) = secret.versions.get(target_vid) {
                return Ok(ver.clone());
            }
            return Err(RustStackError::secretsmanager_not_found(
                "ResourceNotFoundException",
                format!("Version {} not found for secret {}.", target_vid, secret_id),
            ));
        }

        let stage = version_stage_opt.unwrap_or("AWSCURRENT");
        for ver in secret.versions.values() {
            if ver.version_stages.iter().any(|s| s == stage) {
                return Ok(ver.clone());
            }
        }

        Err(RustStackError::secretsmanager_not_found(
            "ResourceNotFoundException",
            format!(
                "Version stage {} not found for secret {}.",
                stage, secret_id
            ),
        ))
    }

    pub fn put_secret_value(
        &self,
        secret_id: &str,
        secret_string: Option<String>,
        secret_binary: Option<String>,
        client_request_token: Option<String>,
        version_stages_opt: Option<Vec<String>>,
    ) -> Result<(String, String, Vec<String>), RustStackError> {
        let secret_arc = self.find_secret(secret_id).ok_or_else(|| {
            RustStackError::secretsmanager_not_found(
                "ResourceNotFoundException",
                format!("Secret {} not found.", secret_id),
            )
        })?;

        let mut secret = secret_arc.write();
        let now = Utc::now();
        let new_vid = client_request_token.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let stages = version_stages_opt.unwrap_or_else(|| vec!["AWSCURRENT".to_string()]);

        // If stages include AWSCURRENT, remove AWSCURRENT from old current and give it AWSPREVIOUS
        if stages.contains(&"AWSCURRENT".to_string()) {
            if let Some(old_vid) = secret.current_version_id.clone() {
                if let Some(old_ver) = secret.versions.get_mut(&old_vid) {
                    old_ver.version_stages.retain(|s| s != "AWSCURRENT");
                    if !old_ver.version_stages.contains(&"AWSPREVIOUS".to_string()) {
                        old_ver.version_stages.push("AWSPREVIOUS".to_string());
                    }
                }
            }
            secret.current_version_id = Some(new_vid.clone());
        }

        let new_version = SecretVersion {
            version_id: new_vid.clone(),
            secret_string,
            secret_binary,
            version_stages: stages.clone(),
            created_date: now,
        };

        secret.versions.insert(new_vid.clone(), new_version);
        secret.last_changed_date = Some(now);

        Ok((secret.arn.clone(), new_vid, stages))
    }

    pub fn update_secret(
        &self,
        secret_id: &str,
        description: Option<String>,
        kms_key_id: Option<String>,
        secret_string: Option<String>,
        secret_binary: Option<String>,
        client_request_token: Option<String>,
    ) -> Result<String, RustStackError> {
        let secret_arc = self.find_secret(secret_id).ok_or_else(|| {
            RustStackError::secretsmanager_not_found(
                "ResourceNotFoundException",
                format!("Secret {} not found.", secret_id),
            )
        })?;

        let mut secret = secret_arc.write();
        if let Some(d) = description {
            secret.description = Some(d);
        }
        if let Some(k) = kms_key_id {
            secret.kms_key_id = Some(k);
        }

        if secret_string.is_some() || secret_binary.is_some() {
            let new_vid = client_request_token.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let now = Utc::now();

            if let Some(old_vid) = secret.current_version_id.clone() {
                if let Some(old_ver) = secret.versions.get_mut(&old_vid) {
                    old_ver.version_stages.retain(|s| s != "AWSCURRENT");
                    old_ver.version_stages.push("AWSPREVIOUS".to_string());
                }
            }

            let ver = SecretVersion {
                version_id: new_vid.clone(),
                secret_string,
                secret_binary,
                version_stages: vec!["AWSCURRENT".to_string()],
                created_date: now,
            };

            secret.versions.insert(new_vid.clone(), ver);
            secret.current_version_id = Some(new_vid);
            secret.last_changed_date = Some(now);
        }

        Ok(secret.arn.clone())
    }

    pub fn delete_secret(
        &self,
        secret_id: &str,
        force_delete: bool,
    ) -> Result<(String, String, Option<i64>), RustStackError> {
        let secret_arc = self.find_secret(secret_id).ok_or_else(|| {
            RustStackError::secretsmanager_not_found(
                "ResourceNotFoundException",
                format!("Secret {} not found.", secret_id),
            )
        })?;

        let name = secret_arc.read().name.clone();
        let arn = secret_arc.read().arn.clone();

        if force_delete {
            self.secrets_by_name.remove(&name);
            self.secrets_by_arn.remove(&arn);
            return Ok((arn, name, None));
        }

        let now = Utc::now();
        secret_arc.write().deleted_date = Some(now);
        let deletion_date = (now + chrono::Duration::days(30)).timestamp();
        Ok((arn, name, Some(deletion_date)))
    }

    pub fn describe_secret(&self, secret_id: &str) -> Result<Secret, RustStackError> {
        let secret_arc = self.find_secret(secret_id).ok_or_else(|| {
            RustStackError::secretsmanager_not_found(
                "ResourceNotFoundException",
                format!("Secret {} not found.", secret_id),
            )
        })?;

        let s = secret_arc.read().clone();
        Ok(s)
    }

    pub fn list_secrets(&self, _max_results: Option<usize>) -> Result<Vec<Secret>, RustStackError> {
        let mut list = Vec::new();
        for item in self.secrets_by_name.iter() {
            let secret = item.value().read().clone();
            if secret.deleted_date.is_none() {
                list.push(secret);
            }
        }
        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }
}
