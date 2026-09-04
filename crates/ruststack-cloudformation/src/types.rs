use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum StackStatus {
    #[default]
    #[serde(rename = "CREATE_IN_PROGRESS")]
    CreateInProgress,
    #[serde(rename = "CREATE_COMPLETE")]
    CreateComplete,
    #[serde(rename = "CREATE_FAILED")]
    CreateFailed,
    #[serde(rename = "UPDATE_IN_PROGRESS")]
    UpdateInProgress,
    #[serde(rename = "UPDATE_COMPLETE")]
    UpdateComplete,
    #[serde(rename = "DELETE_IN_PROGRESS")]
    DeleteInProgress,
    #[serde(rename = "DELETE_COMPLETE")]
    DeleteComplete,
    #[serde(rename = "DELETE_FAILED")]
    DeleteFailed,
}

impl std::fmt::Display for StackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateInProgress => write!(f, "CREATE_IN_PROGRESS"),
            Self::CreateComplete => write!(f, "CREATE_COMPLETE"),
            Self::CreateFailed => write!(f, "CREATE_FAILED"),
            Self::UpdateInProgress => write!(f, "UPDATE_IN_PROGRESS"),
            Self::UpdateComplete => write!(f, "UPDATE_COMPLETE"),
            Self::DeleteInProgress => write!(f, "DELETE_IN_PROGRESS"),
            Self::DeleteComplete => write!(f, "DELETE_COMPLETE"),
            Self::DeleteFailed => write!(f, "DELETE_FAILED"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub parameter_key: String,
    pub parameter_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub output_key: String,
    pub output_value: String,
    pub description: Option<String>,
    pub export_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackResource {
    pub logical_resource_id: String,
    pub physical_resource_id: String,
    pub resource_type: String,
    pub resource_status: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackEvent {
    pub event_id: String,
    pub stack_id: String,
    pub stack_name: String,
    pub logical_resource_id: String,
    pub physical_resource_id: String,
    pub resource_type: String,
    pub timestamp: DateTime<Utc>,
    pub resource_status: String,
    pub resource_status_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredStack {
    pub stack_id: String,
    pub stack_name: String,
    pub template_body: String,
    pub status: StackStatus,
    pub status_reason: Option<String>,
    pub creation_time: DateTime<Utc>,
    pub last_updated_time: Option<DateTime<Utc>>,
    pub parameters: Vec<Parameter>,
    pub outputs: Vec<Output>,
    pub resources: Vec<StackResource>,
    pub events: Vec<StackEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CloudFormationStateSnapshot {
    pub stacks: HashMap<String, StoredStack>,
}
