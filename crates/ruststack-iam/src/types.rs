use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IamRole {
    pub role_name: String,
    pub role_id: String,
    pub arn: String,
    pub path: String,
    pub assume_role_policy_document: String,
    pub description: Option<String>,
    pub create_date: String,
    pub max_session_duration: i32,
    pub attached_policies: Vec<String>, // list of policy ARNs
    pub inline_policies: HashMap<String, String>, // policy_name -> document
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IamPolicy {
    pub policy_name: String,
    pub policy_id: String,
    pub arn: String,
    pub path: String,
    pub default_version_id: String,
    pub policy_document: String,
    pub description: Option<String>,
    pub create_date: String,
    pub update_date: String,
    pub attachment_count: i32,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IamUser {
    pub user_name: String,
    pub user_id: String,
    pub arn: String,
    pub path: String,
    pub create_date: String,
    pub attached_policies: Vec<String>,
    pub inline_policies: HashMap<String, String>,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IamAccessKey {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub user_name: String,
    pub status: String,
    pub create_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IamStateSnapshot {
    pub roles: Vec<IamRole>,
    pub policies: Vec<IamPolicy>,
    pub users: Vec<IamUser>,
    pub access_keys: Vec<IamAccessKey>,
}
