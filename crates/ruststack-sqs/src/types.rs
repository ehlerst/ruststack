use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttributeValue {
    pub data_type: String,
    pub string_value: Option<String>,
    pub binary_value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SqsMessage {
    pub message_id: String,
    pub receipt_handle: String,
    pub md5_of_body: String,
    pub body: String,
    pub attributes: HashMap<String, String>,
    pub message_attributes: HashMap<String, MessageAttributeValue>,
    pub receive_count: u32,
    pub sent_timestamp: DateTime<Utc>,
    pub first_received_timestamp: Option<DateTime<Utc>>,
    pub visible_at: std::time::Instant,
    pub message_group_id: Option<String>,
    pub message_deduplication_id: Option<String>,
    pub sequence_number: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct QueueAttributes {
    pub visibility_timeout: u32,        // seconds, default 30
    pub delay_seconds: u32,             // seconds, default 0
    pub receive_wait_time_seconds: u32, // seconds, default 0
    pub max_message_size: u32,          // bytes, default 262144
    pub message_retention_period: u32,  // seconds, default 345600
    pub is_fifo: bool,
    pub content_based_deduplication: bool,
    pub redrive_policy: Option<String>,
    pub created_timestamp: DateTime<Utc>,
    pub last_modified_timestamp: DateTime<Utc>,
}

impl Default for QueueAttributes {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            visibility_timeout: 30,
            delay_seconds: 0,
            receive_wait_time_seconds: 0,
            max_message_size: 262144,
            message_retention_period: 345600,
            is_fifo: false,
            content_based_deduplication: false,
            redrive_policy: None,
            created_timestamp: now,
            last_modified_timestamp: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageBatchEntry {
    pub id: String,
    pub message_body: String,
    pub delay_seconds: Option<u32>,
    pub message_attributes: Option<HashMap<String, MessageAttributeValue>>,
    pub message_group_id: Option<String>,
    pub message_deduplication_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageBatchResultEntry {
    pub id: String,
    pub message_id: String,
    pub md5_of_message_body: String,
    pub sequence_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMessageBatchEntry {
    pub id: String,
    pub receipt_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMessageVisibilityBatchEntry {
    pub id: String,
    pub receipt_handle: String,
    pub visibility_timeout: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchErrorEntry {
    pub id: String,
    pub sender_fault: bool,
    pub code: String,
    pub message: String,
}
