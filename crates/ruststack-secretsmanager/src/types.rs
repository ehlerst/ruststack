use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretVersion {
    pub version_id: String,
    pub secret_string: Option<String>,
    pub secret_binary: Option<String>,
    pub version_stages: Vec<String>,
    pub created_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub arn: String,
    pub name: String,
    pub description: Option<String>,
    pub kms_key_id: Option<String>,
    pub deleted_date: Option<DateTime<Utc>>,
    pub created_date: DateTime<Utc>,
    pub last_accessed_date: Option<DateTime<Utc>>,
    pub last_changed_date: Option<DateTime<Utc>>,
    pub versions: HashMap<String, SecretVersion>,
    pub current_version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSecretRequest {
    pub name: String,
    pub description: Option<String>,
    pub kms_key_id: Option<String>,
    pub secret_string: Option<String>,
    pub secret_binary: Option<String>,
    pub client_request_token: Option<String>,
}
