use crate::query::evaluate_expression;
use crate::table::Table;
use crate::types::{
    AttributeDefinition, AttributeValue, DynamoDbSnapshot, GlobalSecondaryIndexDescription,
    KeySchemaElement, LocalSecondaryIndexDescription, QueryOutput, TableDescription, TableSnapshot,
};
use dashmap::DashMap;
use parking_lot::RwLock;
use ruststack_core::RustStackError;
use std::collections::HashMap;
use std::sync::Arc;

pub struct DynamoDbEngine {
    tables: DashMap<String, Arc<RwLock<Table>>>,
    account_id: String,
    region: String,
}

impl DynamoDbEngine {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            tables: DashMap::new(),
            account_id,
            region,
        }
    }

    pub fn format_table_arn(&self, table_name: &str) -> String {
        format!(
            "arn:aws:dynamodb:{}:{}:table/{}",
            self.region, self.account_id, table_name
        )
    }

    pub fn create_table(
        &self,
        table_name: String,
        key_schema: Vec<KeySchemaElement>,
        attribute_definitions: Vec<AttributeDefinition>,
        billing_mode: Option<String>,
        gsis: Option<Vec<GlobalSecondaryIndexDescription>>,
        lsis: Option<Vec<LocalSecondaryIndexDescription>>,
        stream_specification: Option<crate::types::StreamSpecification>,
    ) -> Result<TableDescription, RustStackError> {
        if self.tables.contains_key(&table_name) {
            return Err(RustStackError::dynamodb_bad_request(
                "ResourceInUseException",
                format!("Table already exists: {}", table_name),
            ));
        }

        let arn = self.format_table_arn(&table_name);
        let mut table = Table::new(
            table_name.clone(),
            arn,
            key_schema,
            attribute_definitions,
            billing_mode,
            gsis,
            lsis,
        )?;

        if let Some(ref spec) = stream_specification {
            if spec.stream_enabled {
                let stream_label = format!(
                    "{}-stream",
                    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3f")
                );
                let stream_arn = format!("{}/stream/{}", table.description.table_arn, stream_label);
                table.description.stream_specification = Some(spec.clone());
                table.description.latest_stream_arn = Some(stream_arn);
                table.description.latest_stream_label = Some(stream_label);
            }
        }

        let desc = table.description.clone();
        self.tables.insert(table_name, Arc::new(RwLock::new(table)));
        Ok(desc)
    }

    pub fn delete_table(&self, table_name: &str) -> Result<TableDescription, RustStackError> {
        let (_, table_arc) = self.tables.remove(table_name).ok_or_else(|| {
            RustStackError::dynamodb_not_found(
                "ResourceNotFoundException",
                format!(
                    "Cannot do operations on a non-existent table: {}",
                    table_name
                ),
            )
        })?;

        let mut desc = table_arc.read().description.clone();
        desc.table_status = "DELETING".to_string();
        Ok(desc)
    }

    pub fn describe_table(&self, table_name: &str) -> Result<TableDescription, RustStackError> {
        let table_arc = self.tables.get(table_name).ok_or_else(|| {
            RustStackError::dynamodb_not_found(
                "ResourceNotFoundException",
                format!(
                    "Cannot do operations on a non-existent table: {}",
                    table_name
                ),
            )
        })?;

        let desc = table_arc.read().description.clone();
        Ok(desc)
    }

    pub fn list_tables(&self) -> Vec<String> {
        let mut list: Vec<String> = self.tables.iter().map(|t| t.key().clone()).collect();
        list.sort();
        list
    }

    pub fn put_item(
        &self,
        table_name: &str,
        item: HashMap<String, AttributeValue>,
        condition_expr: Option<&str>,
        attr_names: Option<&HashMap<String, String>>,
        attr_values: Option<&HashMap<String, AttributeValue>>,
    ) -> Result<Option<HashMap<String, AttributeValue>>, RustStackError> {
        let table_arc = self.tables.get(table_name).ok_or_else(|| {
            RustStackError::dynamodb_not_found(
                "ResourceNotFoundException",
                format!(
                    "Cannot do operations on a non-existent table: {}",
                    table_name
                ),
            )
        })?;

        let mut table = table_arc.write();

        // Check condition expression if present
        if let Some(expr) = condition_expr {
            let pk = table.extract_primary_key(&item)?;
            let existing = table.items.get(&pk);
            let dummy_empty = HashMap::new();
            let check_item = existing.unwrap_or(&dummy_empty);

            if !evaluate_expression(expr, check_item, attr_names, attr_values) {
                return Err(RustStackError::dynamodb_bad_request(
                    "ConditionalCheckFailedException",
                    "The conditional request failed.",
                ));
            }
        }

        table.put_item(item)
    }

    pub fn get_item(
        &self,
        table_name: &str,
        key: &HashMap<String, AttributeValue>,
        projection_expr: Option<&str>,
        attr_names: Option<&HashMap<String, String>>,
    ) -> Result<Option<HashMap<String, AttributeValue>>, RustStackError> {
        let table_arc = self.tables.get(table_name).ok_or_else(|| {
            RustStackError::dynamodb_not_found(
                "ResourceNotFoundException",
                format!(
                    "Cannot do operations on a non-existent table: {}",
                    table_name
                ),
            )
        })?;

        let table = table_arc.read();
        let item_opt = table.get_item(key)?;

        if let (Some(item), Some(proj)) = (item_opt.as_ref(), projection_expr) {
            let filtered = project_item(item, proj, attr_names);
            Ok(Some(filtered))
        } else {
            Ok(item_opt)
        }
    }

    pub fn delete_item(
        &self,
        table_name: &str,
        key: &HashMap<String, AttributeValue>,
        condition_expr: Option<&str>,
        attr_names: Option<&HashMap<String, String>>,
        attr_values: Option<&HashMap<String, AttributeValue>>,
    ) -> Result<Option<HashMap<String, AttributeValue>>, RustStackError> {
        let table_arc = self.tables.get(table_name).ok_or_else(|| {
            RustStackError::dynamodb_not_found(
                "ResourceNotFoundException",
                format!(
                    "Cannot do operations on a non-existent table: {}",
                    table_name
                ),
            )
        })?;

        let mut table = table_arc.write();

        if let Some(expr) = condition_expr {
            let pk = table.extract_primary_key(key)?;
            let existing = table.items.get(&pk);
            let dummy_empty = HashMap::new();
            let check_item = existing.unwrap_or(&dummy_empty);

            if !evaluate_expression(expr, check_item, attr_names, attr_values) {
                return Err(RustStackError::dynamodb_bad_request(
                    "ConditionalCheckFailedException",
                    "The conditional request failed.",
                ));
            }
        }

        table.delete_item(key)
    }

    pub fn update_item(
        &self,
        table_name: &str,
        key: &HashMap<String, AttributeValue>,
        update_expr: Option<&str>,
        attr_names: Option<&HashMap<String, String>>,
        attr_values: Option<&HashMap<String, AttributeValue>>,
    ) -> Result<HashMap<String, AttributeValue>, RustStackError> {
        let table_arc = self.tables.get(table_name).ok_or_else(|| {
            RustStackError::dynamodb_not_found(
                "ResourceNotFoundException",
                format!(
                    "Cannot do operations on a non-existent table: {}",
                    table_name
                ),
            )
        })?;

        let mut table = table_arc.write();
        let pk = table.extract_primary_key(key)?;

        let mut item = table.items.get(&pk).cloned().unwrap_or_else(|| key.clone());

        if let Some(expr) = update_expr {
            apply_update_expression(&mut item, expr, attr_names, attr_values)?;
        }

        table.items.insert(pk, item.clone());
        table.description.item_count = table.items.len() as i64;
        Ok(item)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn query(
        &self,
        table_name: &str,
        _index_name: Option<&str>,
        key_condition_expr: &str,
        filter_expr: Option<&str>,
        scan_index_forward: Option<bool>,
        limit: Option<usize>,
        attr_names: Option<&HashMap<String, String>>,
        attr_values: Option<&HashMap<String, AttributeValue>>,
    ) -> Result<QueryOutput, RustStackError> {
        let table_arc = self.tables.get(table_name).ok_or_else(|| {
            RustStackError::dynamodb_not_found(
                "ResourceNotFoundException",
                format!(
                    "Cannot do operations on a non-existent table: {}",
                    table_name
                ),
            )
        })?;

        let table = table_arc.read();
        let mut matched = Vec::new();
        let mut scanned_count = 0;

        let max_items = limit.unwrap_or(usize::MAX);
        let forward = scan_index_forward.unwrap_or(true);

        for item in table.items.values() {
            scanned_count += 1;
            if evaluate_expression(key_condition_expr, item, attr_names, attr_values) {
                let matches_filter = match filter_expr {
                    Some(f) => evaluate_expression(f, item, attr_names, attr_values),
                    None => true,
                };

                if matches_filter {
                    matched.push(item.clone());
                    if matched.len() >= max_items {
                        break;
                    }
                }
            }
        }

        if !forward {
            matched.reverse();
        }

        let count = matched.len();
        Ok(QueryOutput {
            items: matched,
            count,
            scanned_count,
        })
    }

    pub fn scan(
        &self,
        table_name: &str,
        _index_name: Option<&str>,
        filter_expr: Option<&str>,
        limit: Option<usize>,
        attr_names: Option<&HashMap<String, String>>,
        attr_values: Option<&HashMap<String, AttributeValue>>,
    ) -> Result<QueryOutput, RustStackError> {
        let table_arc = self.tables.get(table_name).ok_or_else(|| {
            RustStackError::dynamodb_not_found(
                "ResourceNotFoundException",
                format!(
                    "Cannot do operations on a non-existent table: {}",
                    table_name
                ),
            )
        })?;

        let table = table_arc.read();
        let mut matched = Vec::new();
        let mut scanned_count = 0;
        let max_items = limit.unwrap_or(usize::MAX);

        for item in table.items.values() {
            scanned_count += 1;
            let matches_filter = match filter_expr {
                Some(f) => evaluate_expression(f, item, attr_names, attr_values),
                None => true,
            };

            if matches_filter {
                matched.push(item.clone());
                if matched.len() >= max_items {
                    break;
                }
            }
        }

        let count = matched.len();
        Ok(QueryOutput {
            items: matched,
            count,
            scanned_count,
        })
    }

    pub fn reset(&self) {
        self.tables.clear();
    }

    pub fn list_streams(
        &self,
        table_name: Option<&str>,
    ) -> Result<Vec<crate::types::StreamDescription>, RustStackError> {
        let mut streams = Vec::new();
        for entry in self.tables.iter() {
            let table = entry.value().read();
            if let Some(ref name) = table_name {
                if table.description.table_name != *name {
                    continue;
                }
            }
            if let Some(ref arn) = table.description.latest_stream_arn {
                let label = table
                    .description
                    .latest_stream_label
                    .clone()
                    .unwrap_or_default();
                let view_type = table
                    .description
                    .stream_specification
                    .as_ref()
                    .and_then(|s| s.stream_view_type.clone())
                    .unwrap_or_else(|| "NEW_AND_OLD_IMAGES".to_string());

                streams.push(crate::types::StreamDescription {
                    stream_arn: arn.clone(),
                    stream_label: label,
                    stream_status: "ENABLED".to_string(),
                    stream_view_type: view_type,
                    creation_request_date_time: table.description.creation_date_time,
                    table_name: table.description.table_name.clone(),
                    key_schema: table.description.key_schema.clone(),
                    shards: vec![crate::types::Shard {
                        shard_id: "shardId-00000000000000000000-00000001".to_string(),
                        sequence_number_range: crate::types::SequenceNumberRange {
                            starting_sequence_number: "000000000000000000001".to_string(),
                            ending_sequence_number: None,
                        },
                        parent_shard_id: None,
                    }],
                    last_evaluated_shard_id: None,
                });
            }
        }
        Ok(streams)
    }

    pub fn describe_stream(
        &self,
        stream_arn: &str,
    ) -> Result<crate::types::StreamDescription, RustStackError> {
        for entry in self.tables.iter() {
            let table = entry.value().read();
            if table.description.latest_stream_arn.as_deref() == Some(stream_arn) {
                let label = table
                    .description
                    .latest_stream_label
                    .clone()
                    .unwrap_or_default();
                let view_type = table
                    .description
                    .stream_specification
                    .as_ref()
                    .and_then(|s| s.stream_view_type.clone())
                    .unwrap_or_else(|| "NEW_AND_OLD_IMAGES".to_string());

                return Ok(crate::types::StreamDescription {
                    stream_arn: stream_arn.to_string(),
                    stream_label: label,
                    stream_status: "ENABLED".to_string(),
                    stream_view_type: view_type,
                    creation_request_date_time: table.description.creation_date_time,
                    table_name: table.description.table_name.clone(),
                    key_schema: table.description.key_schema.clone(),
                    shards: vec![crate::types::Shard {
                        shard_id: "shardId-00000000000000000000-00000001".to_string(),
                        sequence_number_range: crate::types::SequenceNumberRange {
                            starting_sequence_number: "000000000000000000001".to_string(),
                            ending_sequence_number: None,
                        },
                        parent_shard_id: None,
                    }],
                    last_evaluated_shard_id: None,
                });
            }
        }
        Err(RustStackError::dynamodb_not_found(
            "ResourceNotFoundException",
            format!("Cannot find stream with ARN: {}", stream_arn),
        ))
    }

    pub fn get_shard_iterator(
        &self,
        stream_arn: &str,
        shard_id: &str,
        iterator_type: &str,
        sequence_number: Option<&str>,
    ) -> Result<String, RustStackError> {
        let _ = self.describe_stream(stream_arn)?;
        let start_pos = match iterator_type {
            "TRIM_HORIZON" => 0,
            "LATEST" => {
                let mut pos = 0;
                for entry in self.tables.iter() {
                    let t = entry.value().read();
                    if t.description.latest_stream_arn.as_deref() == Some(stream_arn) {
                        pos = t.stream_records.len();
                        break;
                    }
                }
                pos
            }
            "AT_SEQUENCE_NUMBER" | "AFTER_SEQUENCE_NUMBER" => {
                let target_seq = sequence_number.unwrap_or("0");
                let mut pos = 0;
                for entry in self.tables.iter() {
                    let t = entry.value().read();
                    if t.description.latest_stream_arn.as_deref() == Some(stream_arn) {
                        for (idx, r) in t.stream_records.iter().enumerate() {
                            if r.dynamodb.sequence_number == target_seq {
                                pos = if iterator_type == "AFTER_SEQUENCE_NUMBER" {
                                    idx + 1
                                } else {
                                    idx
                                };
                                break;
                            }
                        }
                        break;
                    }
                }
                pos
            }
            _ => 0,
        };

        let payload = serde_json::json!({
            "a": stream_arn,
            "s": shard_id,
            "p": start_pos
        });
        Ok(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            payload.to_string().as_bytes(),
        ))
    }

    pub fn get_records(
        &self,
        shard_iterator: &str,
        limit: Option<usize>,
    ) -> Result<(Vec<crate::types::DynamoDbStreamRecord>, Option<String>), RustStackError> {
        let decoded =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, shard_iterator)
                .map_err(|_| {
                    RustStackError::dynamodb_bad_request(
                        "InvalidArgumentException",
                        "Invalid ShardIterator",
                    )
                })?;
        let payload: serde_json::Value = serde_json::from_slice(&decoded).map_err(|_| {
            RustStackError::dynamodb_bad_request(
                "InvalidArgumentException",
                "Invalid ShardIterator format",
            )
        })?;

        let stream_arn = payload["a"].as_str().unwrap_or("");
        let shard_id = payload["s"].as_str().unwrap_or("");
        let pos = payload["p"].as_u64().unwrap_or(0) as usize;
        let max_limit = limit.unwrap_or(100).min(1000);

        for entry in self.tables.iter() {
            let table = entry.value().read();
            if table.description.latest_stream_arn.as_deref() == Some(stream_arn) {
                let records = if pos < table.stream_records.len() {
                    let end = (pos + max_limit).min(table.stream_records.len());
                    table.stream_records[pos..end].to_vec()
                } else {
                    Vec::new()
                };

                let next_pos = pos + records.len();
                let next_payload = serde_json::json!({
                    "a": stream_arn,
                    "s": shard_id,
                    "p": next_pos
                });
                let next_iter_b64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    next_payload.to_string().as_bytes(),
                );

                return Ok((records, Some(next_iter_b64)));
            }
        }

        Err(RustStackError::dynamodb_not_found(
            "ResourceNotFoundException",
            format!("Stream not found: {}", stream_arn),
        ))
    }

    pub fn dump_state(&self) -> DynamoDbSnapshot {
        self.export_state()
    }

    pub fn export_state(&self) -> DynamoDbSnapshot {
        let mut tables = Vec::new();
        for entry in self.tables.iter() {
            let table = entry.value().read();
            let items: Vec<HashMap<String, AttributeValue>> =
                table.items.values().cloned().collect();
            tables.push(TableSnapshot {
                description: table.description.clone(),
                items,
                stream_records: table.stream_records.clone(),
            });
        }
        tables.sort_by(|a, b| a.description.table_name.cmp(&b.description.table_name));
        DynamoDbSnapshot { tables }
    }

    pub fn load_state(&self, snapshot: DynamoDbSnapshot) {
        self.tables.clear();
        for t_snap in snapshot.tables {
            let desc = t_snap.description;
            if let Ok(mut table) = Table::new(
                desc.table_name.clone(),
                desc.table_arn.clone(),
                desc.key_schema.clone(),
                desc.attribute_definitions.clone(),
                desc.billing_mode_summary.map(|b| b.billing_mode),
                desc.global_secondary_indexes.clone(),
                desc.local_secondary_indexes.clone(),
            ) {
                table.description.creation_date_time = desc.creation_date_time;
                table.description.stream_specification = desc.stream_specification;
                table.description.latest_stream_arn = desc.latest_stream_arn;
                table.description.latest_stream_label = desc.latest_stream_label;
                table.stream_records = t_snap.stream_records;
                for item in t_snap.items {
                    if let Ok(pk) = table.extract_primary_key(&item) {
                        table.items.insert(pk, item);
                    }
                }
                table.description.item_count = table.items.len() as i64;
                self.tables
                    .insert(desc.table_name, Arc::new(RwLock::new(table)));
            }
        }
    }
}

fn project_item(
    item: &HashMap<String, AttributeValue>,
    projection: &str,
    attr_names: Option<&HashMap<String, String>>,
) -> HashMap<String, AttributeValue> {
    let mut filtered = HashMap::new();
    for field in projection.split(',') {
        let field_clean = field.trim();
        let real_name = if let Some(map) = attr_names {
            map.get(field_clean)
                .map(|s| s.as_str())
                .unwrap_or(field_clean)
        } else {
            field_clean
        };

        if let Some(val) = item.get(real_name) {
            filtered.insert(real_name.to_string(), val.clone());
        }
    }
    filtered
}

fn apply_update_expression(
    item: &mut HashMap<String, AttributeValue>,
    expr: &str,
    attr_names: Option<&HashMap<String, String>>,
    attr_values: Option<&HashMap<String, AttributeValue>>,
) -> Result<(), RustStackError> {
    let expr = expr.trim();
    // Support SET a = :val, b = :val2 and REMOVE c, d
    if let Some(set_part) = expr
        .strip_prefix("SET ")
        .or_else(|| expr.strip_prefix("set "))
    {
        for assignment in set_part.split(',') {
            let parts: Vec<&str> = assignment.split('=').collect();
            if parts.len() == 2 {
                let left = parts[0].trim();
                let right = parts[1].trim();

                let real_name = if let Some(map) = attr_names {
                    map.get(left).map(|s| s.as_str()).unwrap_or(left)
                } else {
                    left
                };

                if let Some(map) = attr_values {
                    if let Some(val) = map.get(right) {
                        item.insert(real_name.to_string(), val.clone());
                    }
                }
            }
        }
    } else if let Some(rem_part) = expr
        .strip_prefix("REMOVE ")
        .or_else(|| expr.strip_prefix("remove "))
    {
        for field in rem_part.split(',') {
            let field_clean = field.trim();
            let real_name = if let Some(map) = attr_names {
                map.get(field_clean)
                    .map(|s| s.as_str())
                    .unwrap_or(field_clean)
            } else {
                field_clean
            };
            item.remove(real_name);
        }
    }

    Ok(())
}
