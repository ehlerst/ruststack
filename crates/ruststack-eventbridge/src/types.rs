use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBus {
    pub name: String,
    pub arn: String,
    pub policy: Option<String>,
    pub created_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub arn: String,
    pub event_bus_name: String,
    pub event_pattern: Option<String>,
    pub state: String, // "ENABLED" | "DISABLED"
    pub description: Option<String>,
    pub schedule_expression: Option<String>,
    pub created_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub id: String,
    pub arn: String,
    pub input: Option<String>,
    pub input_path: Option<String>,
    pub role_arn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutEventsRequestEntry {
    pub time: Option<DateTime<Utc>>,
    pub source: Option<String>,
    pub resources: Option<Vec<String>>,
    pub detail_type: Option<String>,
    pub detail: Option<String>,
    pub event_bus_name: Option<String>,
    pub trace_header: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutEventsResultEntry {
    pub event_id: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutTargetsResultEntry {
    pub target_id: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveTargetsResultEntry {
    pub target_id: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

// Snapshot Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSnapshot {
    pub rule: Rule,
    pub targets: Vec<Target>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventBridgeSnapshot {
    pub event_buses: Vec<EventBus>,
    pub rules: Vec<RuleSnapshot>,
}
