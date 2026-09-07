use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipe {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Arn")]
    pub arn: String,
    #[serde(rename = "DesiredState")]
    pub desired_state: String,
    #[serde(rename = "CurrentState")]
    pub current_state: String,
    #[serde(rename = "Source")]
    pub source: String,
    #[serde(rename = "Target")]
    pub target: String,
    #[serde(rename = "Enrichment", skip_serializing_if = "Option::is_none")]
    pub enrichment: Option<String>,
    #[serde(rename = "RoleArn")]
    pub role_arn: String,
    #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "SourceParameters", skip_serializing_if = "Option::is_none")]
    pub source_parameters: Option<serde_json::Value>,
    #[serde(rename = "TargetParameters", skip_serializing_if = "Option::is_none")]
    pub target_parameters: Option<serde_json::Value>,
    #[serde(rename = "CreationTime")]
    pub creation_time: String,
    #[serde(rename = "LastModifiedTime")]
    pub last_modified_time: String,
    #[serde(rename = "Tags", default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeSummary {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Arn")]
    pub arn: String,
    #[serde(rename = "DesiredState")]
    pub desired_state: String,
    #[serde(rename = "CurrentState")]
    pub current_state: String,
    #[serde(rename = "Source")]
    pub source: String,
    #[serde(rename = "Target")]
    pub target: String,
    #[serde(rename = "CreationTime")]
    pub creation_time: String,
    #[serde(rename = "LastModifiedTime")]
    pub last_modified_time: String,
}

#[derive(Debug, thiserror::Error)]
pub enum PipesError {
    #[error("Pipe with name '{0}' already exists")]
    PipeAlreadyExists(String),
    #[error("Pipe '{0}' not found")]
    PipeNotFound(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}
