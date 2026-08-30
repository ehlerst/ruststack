use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    pub name: String,
    pub creation_date: DateTime<Utc>,
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub key: String,
    pub size: u64,
    pub etag: String,
    pub last_modified: DateTime<Utc>,
    pub content_type: String,
    pub user_metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ByteRange {
    pub start: u64,
    pub end: Option<u64>,
}

impl ByteRange {
    pub fn parse(header_val: &str, total_size: u64) -> Option<Self> {
        if !header_val.starts_with("bytes=") {
            return None;
        }
        let range_str = &header_val[6..];
        let parts: Vec<&str> = range_str.split('-').collect();
        if parts.len() != 2 {
            return None;
        }

        if parts[0].is_empty() {
            let suffix: u64 = parts[1].parse().ok()?;
            let start = total_size.saturating_sub(suffix);
            Some(Self {
                start,
                end: Some(total_size.saturating_sub(1)),
            })
        } else {
            let start: u64 = parts[0].parse().ok()?;
            if parts[1].is_empty() {
                Some(Self {
                    start,
                    end: Some(total_size.saturating_sub(1)),
                })
            } else {
                let end: u64 = parts[1].parse().ok()?;
                Some(Self {
                    start,
                    end: Some(end.min(total_size.saturating_sub(1))),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedPart {
    pub part_number: i32,
    pub etag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartInfo {
    pub part_number: i32,
    pub etag: String,
    pub size: u64,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct ListObjectsV2Result {
    pub is_truncated: bool,
    pub contents: Vec<ObjectMetadata>,
    pub name: String,
    pub prefix: String,
    pub delimiter: Option<String>,
    pub max_keys: usize,
    pub common_prefixes: Vec<String>,
    pub key_count: usize,
    pub continuation_token: Option<String>,
    pub next_continuation_token: Option<String>,
    pub start_after: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DeleteObjectsResult {
    pub deleted: Vec<String>,
    pub errors: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationFilterRule {
    pub name: String, // "prefix" or "suffix"
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationFilter {
    pub rules: Vec<NotificationFilterRule>,
}

impl NotificationFilter {
    pub fn matches(&self, key: &str) -> bool {
        for rule in &self.rules {
            if rule.name.eq_ignore_ascii_case("prefix") && !key.starts_with(&rule.value) {
                return false;
            }
            if rule.name.eq_ignore_ascii_case("suffix") && !key.ends_with(&rule.value) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueNotificationConfig {
    pub id: String,
    pub queue_arn: String,
    pub events: Vec<String>,
    pub filter: Option<NotificationFilter>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopicNotificationConfig {
    pub id: String,
    pub topic_arn: String,
    pub events: Vec<String>,
    pub filter: Option<NotificationFilter>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BucketNotificationConfig {
    pub queue_configurations: Vec<QueueNotificationConfig>,
    pub topic_configurations: Vec<TopicNotificationConfig>,
    pub eventbridge_enabled: bool,
}

impl BucketNotificationConfig {
    pub fn is_empty(&self) -> bool {
        self.queue_configurations.is_empty()
            && self.topic_configurations.is_empty()
            && !self.eventbridge_enabled
    }
}
