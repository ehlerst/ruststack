use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StateMachineType {
    #[default]
    Standard,
    Express,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StateMachineStatus {
    #[default]
    Active,
    Deleting,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionStatus {
    #[default]
    Running,
    Succeeded,
    Failed,
    TimedOut,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StateMachineListItem {
    pub state_machine_arn: String,
    pub name: String,
    #[serde(rename = "type")]
    pub state_machine_type: StateMachineType,
    pub creation_date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExecutionListItem {
    pub execution_arn: String,
    pub state_machine_arn: String,
    pub name: String,
    pub status: ExecutionStatus,
    pub start_date: f64,
    pub stop_date: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HistoryEvent {
    pub timestamp: f64,
    #[serde(rename = "type")]
    pub event_type: String,
    pub id: i64,
    pub previous_event_id: i64,
    pub execution_started_event_details: Option<serde_json::Value>,
    pub execution_succeeded_event_details: Option<serde_json::Value>,
    pub execution_failed_event_details: Option<serde_json::Value>,
    pub state_entered_event_details: Option<serde_json::Value>,
    pub state_exited_event_details: Option<serde_json::Value>,
}

// ----------------------------------------------------------------------------
// Request / Response types
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateStateMachineRequest {
    pub name: String,
    pub definition: String,
    pub role_arn: String,
    #[serde(rename = "type")]
    pub state_machine_type: Option<StateMachineType>,
    pub logging_configuration: Option<serde_json::Value>,
    pub tracing_configuration: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateStateMachineResponse {
    pub state_machine_arn: String,
    pub creation_date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeStateMachineRequest {
    pub state_machine_arn: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeStateMachineResponse {
    pub state_machine_arn: String,
    pub name: String,
    pub status: StateMachineStatus,
    pub definition: String,
    pub role_arn: String,
    #[serde(rename = "type")]
    pub state_machine_type: StateMachineType,
    pub creation_date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListStateMachinesRequest {
    pub max_results: Option<usize>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListStateMachinesResponse {
    pub state_machines: Vec<StateMachineListItem>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteStateMachineRequest {
    pub state_machine_arn: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StartExecutionRequest {
    pub state_machine_arn: String,
    pub name: Option<String>,
    pub input: Option<String>,
    pub trace_header: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StartExecutionResponse {
    pub execution_arn: String,
    pub start_date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeExecutionRequest {
    pub execution_arn: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeExecutionResponse {
    pub execution_arn: String,
    pub state_machine_arn: String,
    pub name: String,
    pub status: ExecutionStatus,
    pub start_date: f64,
    pub stop_date: Option<f64>,
    pub input: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub cause: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetExecutionHistoryRequest {
    pub execution_arn: String,
    pub max_results: Option<usize>,
    pub next_token: Option<String>,
    pub reverse_order: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct GetExecutionHistoryResponse {
    pub events: Vec<HistoryEvent>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StopExecutionRequest {
    pub execution_arn: String,
    pub error: Option<String>,
    pub cause: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StopExecutionResponse {
    pub stop_date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListExecutionsRequest {
    pub state_machine_arn: String,
    pub status_filter: Option<ExecutionStatus>,
    pub max_results: Option<usize>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListExecutionsResponse {
    pub executions: Vec<ExecutionListItem>,
    pub next_token: Option<String>,
}

// ----------------------------------------------------------------------------
// State and Snapshot Models
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredExecution {
    pub execution_arn: String,
    pub state_machine_arn: String,
    pub name: String,
    pub status: ExecutionStatus,
    pub start_date: f64,
    pub stop_date: Option<f64>,
    pub input: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub cause: Option<String>,
    pub events: Vec<HistoryEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredStateMachine {
    pub arn: String,
    pub name: String,
    pub definition: String,
    pub role_arn: String,
    pub state_machine_type: StateMachineType,
    pub status: StateMachineStatus,
    pub created_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StepFunctionsStateSnapshot {
    pub state_machines: HashMap<String, StoredStateMachine>,
    pub executions: HashMap<String, StoredExecution>,
}
