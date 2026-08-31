use crate::types::{
    AttributeDefinition, AttributeValue, BillingModeSummary, GlobalSecondaryIndexDescription,
    KeySchemaElement, LocalSecondaryIndexDescription, PrimaryKey, TableDescription,
};
use chrono::Utc;
use ruststack_core::RustStackError;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone)]
pub struct SecondaryIndex {
    pub name: String,
    pub hash_key_name: String,
    pub range_key_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub description: TableDescription,
    pub hash_key_name: String,
    pub range_key_name: Option<String>,
    pub items: BTreeMap<PrimaryKey, HashMap<String, AttributeValue>>,
    pub gsis: HashMap<String, SecondaryIndex>,
    pub lsis: HashMap<String, SecondaryIndex>,
    pub stream_records: Vec<crate::types::DynamoDbStreamRecord>,
}

impl Table {
    pub fn new(
        table_name: String,
        table_arn: String,
        key_schema: Vec<KeySchemaElement>,
        attribute_definitions: Vec<AttributeDefinition>,
        billing_mode: Option<String>,
        gsis_desc: Option<Vec<GlobalSecondaryIndexDescription>>,
        lsis_desc: Option<Vec<LocalSecondaryIndexDescription>>,
    ) -> Result<Self, RustStackError> {
        let hash_key_name = key_schema
            .iter()
            .find(|k| k.key_type == "HASH")
            .map(|k| k.attribute_name.clone())
            .ok_or_else(|| {
                RustStackError::dynamodb_bad_request(
                    "ValidationException",
                    "No HASH key specified in KeySchema.",
                )
            })?;

        let range_key_name = key_schema
            .iter()
            .find(|k| k.key_type == "RANGE")
            .map(|k| k.attribute_name.clone());

        let mut gsis = HashMap::new();
        if let Some(ref list) = gsis_desc {
            for g in list {
                let h_name = g
                    .key_schema
                    .iter()
                    .find(|k| k.key_type == "HASH")
                    .map(|k| k.attribute_name.clone())
                    .ok_or_else(|| {
                        RustStackError::dynamodb_bad_request(
                            "ValidationException",
                            format!("GSI {} missing HASH key.", g.index_name),
                        )
                    })?;
                let r_name = g
                    .key_schema
                    .iter()
                    .find(|k| k.key_type == "RANGE")
                    .map(|k| k.attribute_name.clone());

                gsis.insert(
                    g.index_name.clone(),
                    SecondaryIndex {
                        name: g.index_name.clone(),
                        hash_key_name: h_name,
                        range_key_name: r_name,
                    },
                );
            }
        }

        let mut lsis = HashMap::new();
        if let Some(ref list) = lsis_desc {
            for l in list {
                let h_name = l
                    .key_schema
                    .iter()
                    .find(|k| k.key_type == "HASH")
                    .map(|k| k.attribute_name.clone())
                    .unwrap_or_else(|| hash_key_name.clone());
                let r_name = l
                    .key_schema
                    .iter()
                    .find(|k| k.key_type == "RANGE")
                    .map(|k| k.attribute_name.clone());

                lsis.insert(
                    l.index_name.clone(),
                    SecondaryIndex {
                        name: l.index_name.clone(),
                        hash_key_name: h_name,
                        range_key_name: r_name,
                    },
                );
            }
        }

        let now_sec = Utc::now().timestamp() as f64;
        let billing_mode_summary = billing_mode.map(|b| BillingModeSummary {
            billing_mode: b,
            last_update_to_pay_per_request_date_time: Some(now_sec),
        });

        let description = TableDescription {
            table_name,
            table_arn,
            table_status: "ACTIVE".to_string(),
            key_schema,
            attribute_definitions,
            item_count: 0,
            table_size_bytes: 0,
            creation_date_time: now_sec,
            global_secondary_indexes: gsis_desc,
            local_secondary_indexes: lsis_desc,
            billing_mode_summary,
            stream_specification: None,
            latest_stream_arn: None,
            latest_stream_label: None,
        };

        Ok(Self {
            description,
            hash_key_name,
            range_key_name,
            items: BTreeMap::new(),
            gsis,
            lsis,
            stream_records: Vec::new(),
        })
    }

    pub fn extract_primary_key(
        &self,
        map: &HashMap<String, AttributeValue>,
    ) -> Result<PrimaryKey, RustStackError> {
        let hash_val = map.get(&self.hash_key_name).cloned().ok_or_else(|| {
            RustStackError::dynamodb_bad_request(
                "ValidationException",
                format!(
                    "Missing required HASH key {} in item/key.",
                    self.hash_key_name
                ),
            )
        })?;

        let range_val = if let Some(ref r_name) = self.range_key_name {
            let r_val = map.get(r_name).cloned().ok_or_else(|| {
                RustStackError::dynamodb_bad_request(
                    "ValidationException",
                    format!("Missing required RANGE key {} in item/key.", r_name),
                )
            })?;
            Some(r_val)
        } else {
            None
        };

        Ok(PrimaryKey {
            hash_key: hash_val,
            range_key: range_val,
        })
    }

    pub fn extract_key_attributes(
        &self,
        item: &HashMap<String, AttributeValue>,
    ) -> HashMap<String, AttributeValue> {
        let mut keys = HashMap::new();
        if let Some(val) = item.get(&self.hash_key_name) {
            keys.insert(self.hash_key_name.clone(), val.clone());
        }
        if let Some(ref r_name) = self.range_key_name {
            if let Some(val) = item.get(r_name) {
                keys.insert(r_name.clone(), val.clone());
            }
        }
        keys
    }

    pub fn put_item(
        &mut self,
        item: HashMap<String, AttributeValue>,
    ) -> Result<Option<HashMap<String, AttributeValue>>, RustStackError> {
        let pk = self.extract_primary_key(&item)?;
        let keys = self.extract_key_attributes(&item);
        let old = self.items.insert(pk, item.clone());
        self.description.item_count = self.items.len() as i64;

        if let Some(ref spec) = self.description.stream_specification {
            if spec.stream_enabled {
                let event_name = if old.is_some() { "MODIFY" } else { "INSERT" };
                let view_type = spec
                    .stream_view_type
                    .as_deref()
                    .unwrap_or("NEW_AND_OLD_IMAGES");
                let seq = (self.stream_records.len() + 1).to_string();
                let now = Utc::now().timestamp() as f64;

                let (new_img, old_img) = match view_type {
                    "NEW_IMAGE" => (Some(item), None),
                    "OLD_IMAGE" => (None, old.clone()),
                    "KEYS_ONLY" => (None, None),
                    _ => (Some(item), old.clone()),
                };

                self.stream_records
                    .push(crate::types::DynamoDbStreamRecord {
                        event_id: uuid::Uuid::new_v4().to_string(),
                        event_name: event_name.to_string(),
                        event_version: "1.1".to_string(),
                        event_source: "aws:dynamodb".to_string(),
                        aws_region: "us-east-1".to_string(),
                        dynamodb: crate::types::StreamRecord {
                            approximate_creation_date_time: Some(now),
                            keys,
                            new_image: new_img,
                            old_image: old_img,
                            sequence_number: seq,
                            size_bytes: 128,
                            stream_view_type: view_type.to_string(),
                        },
                    });
            }
        }

        Ok(old)
    }

    pub fn get_item(
        &self,
        key: &HashMap<String, AttributeValue>,
    ) -> Result<Option<HashMap<String, AttributeValue>>, RustStackError> {
        let pk = self.extract_primary_key(key)?;
        Ok(self.items.get(&pk).cloned())
    }

    pub fn delete_item(
        &mut self,
        key: &HashMap<String, AttributeValue>,
    ) -> Result<Option<HashMap<String, AttributeValue>>, RustStackError> {
        let pk = self.extract_primary_key(key)?;
        let old = self.items.remove(&pk);
        self.description.item_count = self.items.len() as i64;

        if let Some(ref old_item) = old {
            if let Some(ref spec) = self.description.stream_specification {
                if spec.stream_enabled {
                    let keys = self.extract_key_attributes(old_item);
                    let view_type = spec
                        .stream_view_type
                        .as_deref()
                        .unwrap_or("NEW_AND_OLD_IMAGES");
                    let seq = (self.stream_records.len() + 1).to_string();
                    let now = Utc::now().timestamp() as f64;

                    let old_img = match view_type {
                        "NEW_IMAGE" | "KEYS_ONLY" => None,
                        _ => Some(old_item.clone()),
                    };

                    self.stream_records
                        .push(crate::types::DynamoDbStreamRecord {
                            event_id: uuid::Uuid::new_v4().to_string(),
                            event_name: "REMOVE".to_string(),
                            event_version: "1.1".to_string(),
                            event_source: "aws:dynamodb".to_string(),
                            aws_region: "us-east-1".to_string(),
                            dynamodb: crate::types::StreamRecord {
                                approximate_creation_date_time: Some(now),
                                keys,
                                new_image: None,
                                old_image: old_img,
                                sequence_number: seq,
                                size_bytes: 128,
                                stream_view_type: view_type.to_string(),
                            },
                        });
                }
            }
        }

        Ok(old)
    }
}
