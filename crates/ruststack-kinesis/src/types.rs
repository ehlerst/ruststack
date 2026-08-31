use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StreamStatus {
    Creating,
    Deleting,
    Active,
    Updating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StreamMode {
    #[default]
    Provisioned,
    OnDemand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EncryptionType {
    #[default]
    None,
    Kms,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShardIteratorType {
    #[default]
    TrimHorizon,
    Latest,
    AtSequenceNumber,
    AfterSequenceNumber,
    AtTimestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct StreamModeDetails {
    pub stream_mode: StreamMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HashKeyRange {
    pub starting_hash_key: String,
    pub ending_hash_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SequenceNumberRange {
    pub starting_sequence_number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_sequence_number: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Shard {
    pub shard_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_shard_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjacent_parent_shard_id: Option<String>,
    pub hash_key_range: HashKeyRange,
    pub sequence_number_range: SequenceNumberRange,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct EnhancedMetrics {
    #[serde(default)]
    pub shard_level_metrics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StreamDescription {
    pub stream_name: String,
    #[serde(rename = "StreamARN")]
    pub stream_arn: String,
    pub stream_status: StreamStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_mode_details: Option<StreamModeDetails>,
    pub shards: Vec<Shard>,
    pub has_more_shards: bool,
    pub retention_period_hours: u32,
    pub stream_creation_timestamp: f64,
    pub enhanced_monitoring: Vec<EnhancedMetrics>,
    pub encryption_type: EncryptionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StreamDescriptionSummary {
    pub stream_name: String,
    #[serde(rename = "StreamARN")]
    pub stream_arn: String,
    pub stream_status: StreamStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_mode_details: Option<StreamModeDetails>,
    pub retention_period_hours: u32,
    pub stream_creation_timestamp: f64,
    pub enhanced_monitoring: Vec<EnhancedMetrics>,
    pub encryption_type: EncryptionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    pub open_shard_count: u32,
    pub consumer_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StreamSummary {
    pub stream_name: String,
    #[serde(rename = "StreamARN")]
    pub stream_arn: String,
    pub stream_status: StreamStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_mode_details: Option<StreamModeDetails>,
    pub stream_creation_timestamp: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChildShard {
    pub shard_id: String,
    pub parent_shards: Vec<String>,
    pub hash_key_range: HashKeyRange,
}

// ----------------------------------------------------------------------------
// Requests and Responses
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct CreateStreamRequest {
    pub stream_name: String,
    #[serde(default)]
    pub shard_count: Option<u32>,
    #[serde(default)]
    pub stream_mode_details: Option<StreamModeDetails>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteStreamRequest {
    #[serde(default)]
    pub stream_name: Option<String>,
    #[serde(rename = "StreamARN", default)]
    pub stream_arn: Option<String>,
    #[serde(default)]
    pub enforce_consumer_deletion: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeStreamRequest {
    #[serde(default)]
    pub stream_name: Option<String>,
    #[serde(rename = "StreamARN", default)]
    pub stream_arn: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub exclusive_start_shard_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeStreamResponse {
    pub stream_description: StreamDescription,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeStreamSummaryRequest {
    #[serde(default)]
    pub stream_name: Option<String>,
    #[serde(rename = "StreamARN", default)]
    pub stream_arn: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeStreamSummaryResponse {
    pub stream_description_summary: StreamDescriptionSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListStreamsRequest {
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub exclusive_start_stream_name: Option<String>,
    #[serde(default)]
    pub next_token: Option<String>,
    #[serde(default)]
    pub stream_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListStreamsResponse {
    pub stream_names: Vec<String>,
    pub has_more_streams: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_summaries: Option<Vec<StreamSummary>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PutRecordRequest {
    #[serde(default)]
    pub stream_name: Option<String>,
    #[serde(rename = "StreamARN", default)]
    pub stream_arn: Option<String>,
    pub data: String, // Base64 encoded
    pub partition_key: String,
    #[serde(default)]
    pub explicit_hash_key: Option<String>,
    #[serde(default)]
    pub sequence_number_for_ordering: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PutRecordResponse {
    pub sequence_number: String,
    pub shard_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_type: Option<EncryptionType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PutRecordsRequestEntry {
    pub data: String, // Base64 encoded
    pub partition_key: String,
    #[serde(default)]
    pub explicit_hash_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PutRecordsRequest {
    #[serde(default)]
    pub stream_name: Option<String>,
    #[serde(rename = "StreamARN", default)]
    pub stream_arn: Option<String>,
    pub records: Vec<PutRecordsRequestEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PutRecordsResultEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shard_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PutRecordsResponse {
    pub failed_record_count: usize,
    pub records: Vec<PutRecordsResultEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_type: Option<EncryptionType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct GetShardIteratorRequest {
    #[serde(default)]
    pub stream_name: Option<String>,
    #[serde(rename = "StreamARN", default)]
    pub stream_arn: Option<String>,
    pub shard_id: String,
    pub shard_iterator_type: ShardIteratorType,
    #[serde(default)]
    pub starting_sequence_number: Option<String>,
    #[serde(default)]
    pub timestamp: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetShardIteratorResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shard_iterator: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Record {
    pub data: String, // Base64 encoded
    pub partition_key: String,
    pub sequence_number: String,
    pub approximate_arrival_timestamp: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_type: Option<EncryptionType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetRecordsRequest {
    pub shard_iterator: String,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(rename = "StreamARN", default)]
    pub stream_arn: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetRecordsResponse {
    pub records: Vec<Record>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_shard_iterator: Option<String>,
    pub millis_behind_latest: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_shards: Option<Vec<ChildShard>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct AddTagsToStreamRequest {
    #[serde(default)]
    pub stream_name: Option<String>,
    #[serde(rename = "StreamARN", default)]
    pub stream_arn: Option<String>,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct RemoveTagsFromStreamRequest {
    #[serde(default)]
    pub stream_name: Option<String>,
    #[serde(rename = "StreamARN", default)]
    pub stream_arn: Option<String>,
    pub tag_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListTagsForStreamRequest {
    #[serde(default)]
    pub stream_name: Option<String>,
    #[serde(rename = "StreamARN", default)]
    pub stream_arn: Option<String>,
    #[serde(default)]
    pub exclusive_start_tag_key: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListTagsForStreamResponse {
    pub tags: Vec<Tag>,
    pub has_more_tags: bool,
}

// ----------------------------------------------------------------------------
// Internal storage and Snapshot models
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRecord {
    pub data: String,
    pub partition_key: String,
    pub sequence_number: String,
    pub approximate_arrival_timestamp: f64,
    pub encryption_type: EncryptionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredShard {
    pub shard_id: String,
    pub parent_shard_id: Option<String>,
    pub adjacent_parent_shard_id: Option<String>,
    pub starting_hash_key: String,
    pub ending_hash_key: String,
    pub starting_sequence_number: String,
    pub ending_sequence_number: Option<String>,
    pub records: Vec<StoredRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredStream {
    pub stream_name: String,
    pub arn: String,
    pub stream_status: StreamStatus,
    pub stream_mode: StreamMode,
    pub retention_period_hours: u32,
    pub stream_creation_timestamp: f64,
    pub shards: Vec<StoredShard>,
    pub tags: HashMap<String, String>,
    pub encryption_type: EncryptionType,
    pub key_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KinesisStateSnapshot {
    pub streams: Vec<StoredStream>,
    pub global_seq_counter: u64,
}
