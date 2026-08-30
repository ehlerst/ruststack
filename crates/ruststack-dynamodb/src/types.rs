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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingModeSummary {
    #[serde(rename = "BillingMode")]
    pub billing_mode: String,
    #[serde(rename = "LastUpdateToPayPerRequestDateTime")]
    pub last_update_to_pay_per_request_date_time: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOutput {
    pub items: Vec<HashMap<String, AttributeValue>>,
    pub count: usize,
    pub scanned_count: usize,
}
