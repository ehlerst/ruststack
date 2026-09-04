use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VisibilityConfig {
    pub sampled_requests_enabled: bool,
    pub cloud_watch_metrics_enabled: bool,
    pub metric_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WebACL {
    pub name: String,
    pub id: String,
    pub arn: String,
    pub default_action: serde_json::Value,
    pub description: Option<String>,
    pub rules: Vec<serde_json::Value>,
    pub visibility_config: VisibilityConfig,
    pub capacity: i64,
    pub lock_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WebACLSummary {
    pub name: String,
    pub id: String,
    pub description: Option<String>,
    pub lock_token: String,
    pub arn: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IPSet {
    pub name: String,
    pub id: String,
    pub arn: String,
    pub description: Option<String>,
    pub ip_address_version: String,
    pub addresses: Vec<String>,
    pub lock_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IPSetSummary {
    pub name: String,
    pub id: String,
    pub description: Option<String>,
    pub lock_token: String,
    pub arn: String,
}
