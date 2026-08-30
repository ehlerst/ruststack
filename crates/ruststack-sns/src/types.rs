use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopicAttributes {
    pub display_name: Option<String>,
    pub policy: Option<String>,
    pub delivery_policy: Option<String>,
    pub effective_delivery_policy: Option<String>,
    pub kms_master_key_id: Option<String>,
    pub is_fifo: bool,
    pub content_based_deduplication: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub arn: String,
    pub name: String,
    pub attributes: TopicAttributes,
    pub created_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubscriptionAttributes {
    pub raw_message_delivery: bool,
    pub filter_policy: Option<String>,
    pub redrive_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub subscription_arn: String,
    pub topic_arn: String,
    pub protocol: String,
    pub endpoint: String,
    pub owner: String,
    pub attributes: SubscriptionAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttributeValue {
    pub data_type: String,
    pub string_value: Option<String>,
    pub binary_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishBatchEntry {
    pub id: String,
    pub message: String,
    pub subject: Option<String>,
    pub message_attributes: Option<HashMap<String, MessageAttributeValue>>,
    pub message_deduplication_id: Option<String>,
    pub message_group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishBatchResultEntry {
    pub id: String,
    pub message_id: String,
    pub sequence_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchErrorEntry {
    pub id: String,
    pub code: String,
    pub message: String,
    pub sender_fault: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnsSnapshot {
    pub topics: Vec<Topic>,
    pub subscriptions: Vec<Subscription>,
}
