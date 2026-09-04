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

// Versioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectVersion {
    pub key: String,
    pub version_id: String,
    pub is_latest: bool,
    pub last_modified: DateTime<Utc>,
    pub etag: String,
    pub size: u64,
    pub is_delete_marker: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ListObjectVersionsResult {
    pub name: String,
    pub prefix: String,
    pub key_marker: Option<String>,
    pub version_id_marker: Option<String>,
    pub next_key_marker: Option<String>,
    pub next_version_id_marker: Option<String>,
    pub max_keys: usize,
    pub is_truncated: bool,
    pub versions: Vec<ObjectVersion>,
    pub delete_markers: Vec<ObjectVersion>,
    pub common_prefixes: Vec<String>,
}

// Lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleRule {
    pub id: Option<String>,
    pub status: String, // "Enabled" or "Disabled"
    pub prefix: Option<String>,
    pub expiration_days: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BucketLifecycleConfig {
    pub rules: Vec<LifecycleRule>,
}

// CORS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsRule {
    pub id: Option<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_origins: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub expose_headers: Vec<String>,
    pub max_age_seconds: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BucketCorsConfig {
    pub rules: Vec<CorsRule>,
}

// Snapshot Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredObjectSnapshot {
    pub metadata: ObjectMetadata,
    pub data_base64: String,
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub is_delete_marker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketSnapshot {
    pub info: BucketInfo,
    pub notifications: BucketNotificationConfig,
    pub objects: Vec<StoredObjectSnapshot>,
    #[serde(default)]
    pub versioning: Option<String>,
    #[serde(default)]
    pub lifecycle: Option<BucketLifecycleConfig>,
    #[serde(default)]
    pub cors: Option<BucketCorsConfig>,
    #[serde(default)]
    pub policy: Option<String>,
    #[serde(default)]
    pub tagging: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct S3Snapshot {
    pub buckets: Vec<BucketSnapshot>,
}
