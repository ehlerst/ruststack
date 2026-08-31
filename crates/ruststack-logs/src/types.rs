use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLogGroupRequest {
    pub log_group_name: String,
    pub retention_in_days: Option<i32>,
    pub tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteLogGroupRequest {
    pub log_group_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeLogGroupsRequest {
    pub log_group_name_prefix: Option<String>,
    pub log_group_name_pattern: Option<String>,
    pub limit: Option<usize>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogGroup {
    pub log_group_name: String,
    pub arn: String,
    pub creation_time: i64,
    pub retention_in_days: Option<i32>,
    pub metric_filter_count: i32,
    pub stored_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLogStreamRequest {
    pub log_group_name: String,
    pub log_stream_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteLogStreamRequest {
    pub log_group_name: String,
    pub log_stream_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeLogStreamsRequest {
    pub log_group_name: String,
    pub log_stream_name_prefix: Option<String>,
    pub order_by: Option<String>,
    pub descending: Option<bool>,
    pub limit: Option<usize>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogStream {
    pub log_stream_name: String,
    pub arn: String,
    pub creation_time: i64,
    pub first_event_timestamp: Option<i64>,
    pub last_event_timestamp: Option<i64>,
    pub last_ingress_time: Option<i64>,
    pub upload_sequence_token: Option<String>,
    pub stored_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputLogEvent {
    pub timestamp: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputLogEvent {
    pub timestamp: i64,
    pub message: String,
    pub ingestion_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutLogEventsRequest {
    pub log_group_name: String,
    pub log_stream_name: String,
    pub log_events: Vec<InputLogEvent>,
    pub sequence_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLogEventsRequest {
    pub log_group_name: String,
    pub log_stream_name: String,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub limit: Option<usize>,
    pub next_token: Option<String>,
    pub start_from_head: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterLogEventsRequest {
    pub log_group_name: String,
    pub log_stream_names: Option<Vec<String>>,
    pub log_stream_name_prefix: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub filter_pattern: Option<String>,
    pub limit: Option<usize>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilteredLogEvent {
    pub event_id: String,
    pub log_stream_name: String,
    pub timestamp: i64,
    pub message: String,
    pub ingestion_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredLogEvent {
    pub event_id: String,
    pub timestamp: i64,
    pub message: String,
    pub ingestion_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredLogStream {
    pub stream: LogStream,
    pub events: Vec<StoredLogEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredLogGroup {
    pub group: LogGroup,
    pub tags: HashMap<String, String>,
    pub streams: HashMap<String, StoredLogStream>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogsStateSnapshot {
    pub groups: Vec<StoredLogGroup>,
}
