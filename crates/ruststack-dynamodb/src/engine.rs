use crate::query::evaluate_expression;
use crate::table::Table;
use crate::types::{
    AttributeDefinition, AttributeValue, GlobalSecondaryIndexDescription, KeySchemaElement,
    LocalSecondaryIndexDescription, QueryOutput, TableDescription,
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
    ) -> Result<TableDescription, RustStackError> {
        if self.tables.contains_key(&table_name) {
            return Err(RustStackError::dynamodb_bad_request(
                "ResourceInUseException",
                format!("Table already exists: {}", table_name),
            ));
        }

        let arn = self.format_table_arn(&table_name);
        let table = Table::new(
            table_name.clone(),
            arn,
            key_schema,
            attribute_definitions,
            billing_mode,
            gsis,
            lsis,
        )?;

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

        for (_pk, item) in table.items.iter() {
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

        for (_pk, item) in table.items.iter() {
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
