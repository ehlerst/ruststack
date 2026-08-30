use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expiration: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssumedRoleUser {
    pub arn: String,
    pub assumed_role_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCallerIdentityResult {
    pub account: String,
    pub arn: String,
    pub user_id: String,
}

// Snapshot Structures
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StsSnapshot {
    pub account_id: String,
    pub user_id: String,
    pub arn: String,
}
