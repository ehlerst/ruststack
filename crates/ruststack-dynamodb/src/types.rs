use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttributeValue {
    #[serde(rename = "S")]
    S(String),
    #[serde(rename = "N")]
    N(String),
    #[serde(rename = "B")]
    B(String),
    #[serde(rename = "SS")]
    SS(Vec<String>),
    #[serde(rename = "NS")]
    NS(Vec<String>),
    #[serde(rename = "BS")]
    BS(Vec<String>),
    #[serde(rename = "M")]
    M(HashMap<String, AttributeValue>),
    #[serde(rename = "L")]
    L(Vec<AttributeValue>),
    #[serde(rename = "NULL")]
    Null(bool),
    #[serde(rename = "BOOL")]
    Bool(bool),
}

impl Hash for AttributeValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::S(s) => {
                0u8.hash(state);
                s.hash(state);
            }
            Self::N(n) => {
                1u8.hash(state);
                n.hash(state);
            }
            Self::B(b) => {
                2u8.hash(state);
                b.hash(state);
            }
            Self::Bool(b) => {
                3u8.hash(state);
                b.hash(state);
            }
            Self::Null(n) => {
                4u8.hash(state);
                n.hash(state);
            }
            Self::SS(ss) => {
                5u8.hash(state);
                ss.hash(state);
            }
            Self::NS(ns) => {
                6u8.hash(state);
                ns.hash(state);
            }
            Self::BS(bs) => {
                7u8.hash(state);
                bs.hash(state);
            }
            Self::L(l) => {
                8u8.hash(state);
                l.hash(state);
            }
            Self::M(m) => {
                9u8.hash(state);
                let mut keys: Vec<&String> = m.keys().collect();
                keys.sort();
                for k in keys {
                    k.hash(state);
                    m.get(k).hash(state);
                }
            }
        }
    }
}

impl PartialOrd for AttributeValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AttributeValue {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (AttributeValue::S(a), AttributeValue::S(b)) => a.cmp(b),
            (AttributeValue::N(a), AttributeValue::N(b)) => {
                let na = a.parse::<f64>().unwrap_or(0.0);
                let nb = b.parse::<f64>().unwrap_or(0.0);
                na.partial_cmp(&nb).unwrap_or(Ordering::Equal)
            }
            (AttributeValue::B(a), AttributeValue::B(b)) => a.cmp(b),
            (AttributeValue::Bool(a), AttributeValue::Bool(b)) => a.cmp(b),
            _ => Ordering::Equal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrimaryKey {
    pub hash_key: AttributeValue,
    pub range_key: Option<AttributeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeySchemaElement {
    #[serde(rename = "AttributeName")]
    pub attribute_name: String,
    #[serde(rename = "KeyType")]
    pub key_type: String, // "HASH" or "RANGE"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDefinition {
    #[serde(rename = "AttributeName")]
    pub attribute_name: String,
    #[serde(rename = "AttributeType")]
    pub attribute_type: String, // "S", "N", "B"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projection {
    #[serde(rename = "ProjectionType")]
    pub projection_type: Option<String>, // "ALL", "KEYS_ONLY", "INCLUDE"
    #[serde(rename = "NonKeyAttributes")]
    pub non_key_attributes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSecondaryIndexDescription {
    #[serde(rename = "IndexName")]
    pub index_name: String,
    #[serde(rename = "KeySchema")]
    pub key_schema: Vec<KeySchemaElement>,
    #[serde(rename = "Projection")]
    pub projection: Projection,
    #[serde(rename = "IndexStatus")]
    pub index_status: String,
    #[serde(rename = "IndexSizeBytes")]
    pub index_size_bytes: i64,
    #[serde(rename = "ItemCount")]
    pub item_count: i64,
    #[serde(rename = "IndexArn")]
    pub index_arn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSecondaryIndexDescription {
    #[serde(rename = "IndexName")]
    pub index_name: String,
    #[serde(rename = "KeySchema")]
    pub key_schema: Vec<KeySchemaElement>,
    #[serde(rename = "Projection")]
    pub projection: Projection,
    #[serde(rename = "IndexSizeBytes")]
    pub index_size_bytes: i64,
    #[serde(rename = "ItemCount")]
    pub item_count: i64,
    #[serde(rename = "IndexArn")]
    pub index_arn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDescription {
    #[serde(rename = "TableName")]
    pub table_name: String,
    #[serde(rename = "TableArn")]
    pub table_arn: String,
    #[serde(rename = "TableStatus")]
    pub table_status: String,
    #[serde(rename = "KeySchema")]
    pub key_schema: Vec<KeySchemaElement>,
    #[serde(rename = "AttributeDefinitions")]
    pub attribute_definitions: Vec<AttributeDefinition>,
    #[serde(rename = "ItemCount")]
    pub item_count: i64,
    #[serde(rename = "TableSizeBytes")]
    pub table_size_bytes: i64,
    #[serde(rename = "CreationDateTime")]
    pub creation_date_time: f64,
    #[serde(rename = "GlobalSecondaryIndexes")]
    pub global_secondary_indexes: Option<Vec<GlobalSecondaryIndexDescription>>,
    #[serde(rename = "LocalSecondaryIndexes")]
    pub local_secondary_indexes: Option<Vec<LocalSecondaryIndexDescription>>,
    #[serde(rename = "BillingModeSummary")]
    pub billing_mode_summary: Option<BillingModeSummary>,
    #[serde(rename = "StreamSpecification")]
    pub stream_specification: Option<StreamSpecification>,
    #[serde(rename = "LatestStreamArn")]
    pub latest_stream_arn: Option<String>,
    #[serde(rename = "LatestStreamLabel")]
    pub latest_stream_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingModeSummary {
    #[serde(rename = "BillingMode")]
    pub billing_mode: String,
    #[serde(rename = "LastUpdateToPayPerRequestDateTime")]
    pub last_update_to_pay_per_request_date_time: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSpecification {
    #[serde(rename = "StreamEnabled")]
    pub stream_enabled: bool,
    #[serde(rename = "StreamViewType")]
    pub stream_view_type: Option<String>, // "NEW_IMAGE", "OLD_IMAGE", "NEW_AND_OLD_IMAGES", "KEYS_ONLY"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shard {
    #[serde(rename = "ShardId")]
    pub shard_id: String,
    #[serde(rename = "SequenceNumberRange")]
    pub sequence_number_range: SequenceNumberRange,
    #[serde(rename = "ParentShardId")]
    pub parent_shard_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceNumberRange {
    #[serde(rename = "StartingSequenceNumber")]
    pub starting_sequence_number: String,
    #[serde(rename = "EndingSequenceNumber")]
    pub ending_sequence_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDescription {
    #[serde(rename = "StreamArn")]
    pub stream_arn: String,
    #[serde(rename = "StreamLabel")]
    pub stream_label: String,
    #[serde(rename = "StreamStatus")]
    pub stream_status: String,
    #[serde(rename = "StreamViewType")]
    pub stream_view_type: String,
    #[serde(rename = "CreationRequestDateTime")]
    pub creation_request_date_time: f64,
    #[serde(rename = "TableName")]
    pub table_name: String,
    #[serde(rename = "KeySchema")]
    pub key_schema: Vec<KeySchemaElement>,
    #[serde(rename = "Shards")]
    pub shards: Vec<Shard>,
    #[serde(rename = "LastEvaluatedShardId")]
    pub last_evaluated_shard_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRecord {
    #[serde(rename = "ApproximateCreationDateTime")]
    pub approximate_creation_date_time: Option<f64>,
    #[serde(rename = "Keys")]
    pub keys: HashMap<String, AttributeValue>,
    #[serde(rename = "NewImage")]
    pub new_image: Option<HashMap<String, AttributeValue>>,
    #[serde(rename = "OldImage")]
    pub old_image: Option<HashMap<String, AttributeValue>>,
    #[serde(rename = "SequenceNumber")]
    pub sequence_number: String,
    #[serde(rename = "SizeBytes")]
    pub size_bytes: i64,
    #[serde(rename = "StreamViewType")]
    pub stream_view_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamoDbStreamRecord {
    #[serde(rename = "eventID")]
    pub event_id: String,
    #[serde(rename = "eventName")]
    pub event_name: String, // "INSERT", "MODIFY", "REMOVE"
    #[serde(rename = "eventVersion")]
    pub event_version: String,
    #[serde(rename = "eventSource")]
    pub event_source: String,
    #[serde(rename = "awsRegion")]
    pub aws_region: String,
    #[serde(rename = "dynamodb")]
    pub dynamodb: StreamRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOutput {
    pub items: Vec<HashMap<String, AttributeValue>>,
    pub count: usize,
    pub scanned_count: usize,
}

// Snapshot Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSnapshot {
    pub description: TableDescription,
    pub items: Vec<HashMap<String, AttributeValue>>,
    #[serde(default)]
    pub stream_records: Vec<DynamoDbStreamRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DynamoDbSnapshot {
    pub tables: Vec<TableSnapshot>,
}
