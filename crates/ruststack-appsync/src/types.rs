use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphqlApi {
    #[serde(rename = "apiId")]
    pub api_id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "authenticationType")]
    pub authentication_type: String,
    #[serde(rename = "arn")]
    pub arn: String,
    #[serde(rename = "uris")]
    pub uris: HashMap<String, String>,
    #[serde(rename = "tags", default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub description: Option<String>,
    pub expires: i64,
    #[serde(rename = "deletes")]
    pub deletes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub data_source_type: String,
    #[serde(rename = "dataSourceArn")]
    pub data_source_arn: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AppSyncError {
    #[error("GraphQL API '{0}' already exists")]
    ApiAlreadyExists(String),
    #[error("GraphQL API '{0}' not found")]
    NotFound(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}
