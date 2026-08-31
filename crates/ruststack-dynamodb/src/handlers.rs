use crate::engine::DynamoDbEngine;
use crate::types::{
    AttributeDefinition, AttributeValue, GlobalSecondaryIndexDescription, KeySchemaElement,
    LocalSecondaryIndexDescription,
};
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use http_body_util::BodyExt;
use ruststack_core::RustStackError;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub async fn handle_dynamodb_request(
    engine: Arc<DynamoDbEngine>,
    req: Request<Body>,
) -> Response<Body> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let (parts, body) = req.into_parts();
    let method = parts.method;
    let uri = parts.uri;
    let headers = parts.headers;

    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return make_dynamodb_error_response(
                &RustStackError::BadRequest(e.to_string()),
                &request_id,
            );
        }
    };

    let result = handle_dynamodb_json(
        engine.as_ref(),
        &method,
        &uri,
        &headers,
        &body_bytes,
        &request_id,
    )
    .await;

    match result {
        Ok(res) => res,
        Err(err) => make_dynamodb_error_response(&err, &request_id),
    }
}

pub fn make_dynamodb_error_response(err: &RustStackError, request_id: &str) -> Response<Body> {
    let status = err.status_code();
    let json_err = err.to_dynamodb_json();
    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.0")
        .header("x-amzn-requestid", request_id)
        .body(Body::from(json_err.to_string()))
        .unwrap()
}

fn make_json_response(val: Value, status: StatusCode) -> Result<Response<Body>, RustStackError> {
    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(val.to_string()))
        .map_err(|e| RustStackError::Internal(e.to_string()))
}

async fn handle_dynamodb_json(
    engine: &DynamoDbEngine,
    _method: &Method,
    _uri: &Uri,
    headers: &HeaderMap,
    body: &[u8],
    _request_id: &str,
) -> Result<Response<Body>, RustStackError> {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let action = target
        .strip_prefix("DynamoDB_20120810.")
        .or_else(|| target.strip_prefix("DynamoDB."))
        .or_else(|| target.strip_prefix("DynamoDBStreams_20120810."))
        .or_else(|| target.strip_prefix("DynamoDBStreams."))
        .unwrap_or(target);

    let json_val: Value = if body.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_slice(body).map_err(|e| RustStackError::BadRequest(e.to_string()))?
    };

    match action {
        "CreateTable" => {
            let table_name = json_val
                .get("TableName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "TableName is required.",
                    )
                })?
                .to_string();

            let key_schema: Vec<KeySchemaElement> = json_val
                .get("KeySchema")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let attribute_definitions: Vec<AttributeDefinition> = json_val
                .get("AttributeDefinitions")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let billing_mode = json_val
                .get("BillingMode")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let gsis: Option<Vec<GlobalSecondaryIndexDescription>> = json_val
                .get("GlobalSecondaryIndexes")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let lsis: Option<Vec<LocalSecondaryIndexDescription>> = json_val
                .get("LocalSecondaryIndexes")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let stream_specification: Option<crate::types::StreamSpecification> = json_val
                .get("StreamSpecification")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let table_desc = engine.create_table(
                table_name,
                key_schema,
                attribute_definitions,
                billing_mode,
                gsis,
                lsis,
                stream_specification,
            )?;

            make_json_response(json!({ "TableDescription": table_desc }), StatusCode::OK)
        }

        "DeleteTable" => {
            let table_name = json_val
                .get("TableName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "TableName is required.",
                    )
                })?;

            let desc = engine.delete_table(table_name)?;
            make_json_response(json!({ "TableDescription": desc }), StatusCode::OK)
        }

        "DescribeTable" => {
            let table_name = json_val
                .get("TableName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "TableName is required.",
                    )
                })?;

            let desc = engine.describe_table(table_name)?;
            make_json_response(json!({ "Table": desc }), StatusCode::OK)
        }

        "ListTables" => {
            let names = engine.list_tables();
            make_json_response(json!({ "TableNames": names }), StatusCode::OK)
        }

        "PutItem" => {
            let table_name = json_val
                .get("TableName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "TableName is required.",
                    )
                })?;

            let item_val = json_val.get("Item").ok_or_else(|| {
                RustStackError::dynamodb_bad_request("ValidationException", "Item is required.")
            })?;

            let item: HashMap<String, AttributeValue> = serde_json::from_value(item_val.clone())
                .map_err(|e| {
                    RustStackError::dynamodb_bad_request("ValidationException", e.to_string())
                })?;

            let condition_expr = json_val.get("ConditionExpression").and_then(|v| v.as_str());
            let attr_names: Option<HashMap<String, String>> = json_val
                .get("ExpressionAttributeNames")
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            let attr_values: Option<HashMap<String, AttributeValue>> = json_val
                .get("ExpressionAttributeValues")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let old_item = engine.put_item(
                table_name,
                item,
                condition_expr,
                attr_names.as_ref(),
                attr_values.as_ref(),
            )?;

            let return_values = json_val.get("ReturnValues").and_then(|v| v.as_str());
            let mut resp = json!({});
            if return_values == Some("ALL_OLD") {
                if let Some(old) = old_item {
                    resp["Attributes"] = json!(old);
                }
            }

            make_json_response(resp, StatusCode::OK)
        }

        "GetItem" => {
            let table_name = json_val
                .get("TableName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "TableName is required.",
                    )
                })?;

            let key_val = json_val.get("Key").ok_or_else(|| {
                RustStackError::dynamodb_bad_request("ValidationException", "Key is required.")
            })?;

            let key: HashMap<String, AttributeValue> = serde_json::from_value(key_val.clone())
                .map_err(|e| {
                    RustStackError::dynamodb_bad_request("ValidationException", e.to_string())
                })?;

            let projection_expr = json_val
                .get("ProjectionExpression")
                .and_then(|v| v.as_str());
            let attr_names: Option<HashMap<String, String>> = json_val
                .get("ExpressionAttributeNames")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let item_opt =
                engine.get_item(table_name, &key, projection_expr, attr_names.as_ref())?;
            let mut resp = json!({});
            if let Some(item) = item_opt {
                resp["Item"] = json!(item);
            }

            make_json_response(resp, StatusCode::OK)
        }

        "DeleteItem" => {
            let table_name = json_val
                .get("TableName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "TableName is required.",
                    )
                })?;

            let key_val = json_val.get("Key").ok_or_else(|| {
                RustStackError::dynamodb_bad_request("ValidationException", "Key is required.")
            })?;

            let key: HashMap<String, AttributeValue> = serde_json::from_value(key_val.clone())
                .map_err(|e| {
                    RustStackError::dynamodb_bad_request("ValidationException", e.to_string())
                })?;

            let condition_expr = json_val.get("ConditionExpression").and_then(|v| v.as_str());
            let attr_names: Option<HashMap<String, String>> = json_val
                .get("ExpressionAttributeNames")
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            let attr_values: Option<HashMap<String, AttributeValue>> = json_val
                .get("ExpressionAttributeValues")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let old_item = engine.delete_item(
                table_name,
                &key,
                condition_expr,
                attr_names.as_ref(),
                attr_values.as_ref(),
            )?;

            let return_values = json_val.get("ReturnValues").and_then(|v| v.as_str());
            let mut resp = json!({});
            if return_values == Some("ALL_OLD") {
                if let Some(old) = old_item {
                    resp["Attributes"] = json!(old);
                }
            }

            make_json_response(resp, StatusCode::OK)
        }

        "UpdateItem" => {
            let table_name = json_val
                .get("TableName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "TableName is required.",
                    )
                })?;

            let key_val = json_val.get("Key").ok_or_else(|| {
                RustStackError::dynamodb_bad_request("ValidationException", "Key is required.")
            })?;

            let key: HashMap<String, AttributeValue> = serde_json::from_value(key_val.clone())
                .map_err(|e| {
                    RustStackError::dynamodb_bad_request("ValidationException", e.to_string())
                })?;

            let update_expr = json_val.get("UpdateExpression").and_then(|v| v.as_str());
            let attr_names: Option<HashMap<String, String>> = json_val
                .get("ExpressionAttributeNames")
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            let attr_values: Option<HashMap<String, AttributeValue>> = json_val
                .get("ExpressionAttributeValues")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let updated_item = engine.update_item(
                table_name,
                &key,
                update_expr,
                attr_names.as_ref(),
                attr_values.as_ref(),
            )?;

            let return_values = json_val.get("ReturnValues").and_then(|v| v.as_str());
            let mut resp = json!({});
            if return_values == Some("ALL_NEW") {
                resp["Attributes"] = json!(updated_item);
            }

            make_json_response(resp, StatusCode::OK)
        }

        "Query" => {
            let table_name = json_val
                .get("TableName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "TableName is required.",
                    )
                })?;

            let key_cond_expr = json_val
                .get("KeyConditionExpression")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let filter_expr = json_val.get("FilterExpression").and_then(|v| v.as_str());
            let index_name = json_val.get("IndexName").and_then(|v| v.as_str());
            let scan_forward = json_val.get("ScanIndexForward").and_then(|v| v.as_bool());
            let limit = json_val
                .get("Limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            let attr_names: Option<HashMap<String, String>> = json_val
                .get("ExpressionAttributeNames")
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            let attr_values: Option<HashMap<String, AttributeValue>> = json_val
                .get("ExpressionAttributeValues")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let output = engine.query(
                table_name,
                index_name,
                key_cond_expr,
                filter_expr,
                scan_forward,
                limit,
                attr_names.as_ref(),
                attr_values.as_ref(),
            )?;

            make_json_response(
                json!({
                    "Items": output.items,
                    "Count": output.count,
                    "ScannedCount": output.scanned_count
                }),
                StatusCode::OK,
            )
        }

        "Scan" => {
            let table_name = json_val
                .get("TableName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "TableName is required.",
                    )
                })?;

            let filter_expr = json_val.get("FilterExpression").and_then(|v| v.as_str());
            let index_name = json_val.get("IndexName").and_then(|v| v.as_str());
            let limit = json_val
                .get("Limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            let attr_names: Option<HashMap<String, String>> = json_val
                .get("ExpressionAttributeNames")
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            let attr_values: Option<HashMap<String, AttributeValue>> = json_val
                .get("ExpressionAttributeValues")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let output = engine.scan(
                table_name,
                index_name,
                filter_expr,
                limit,
                attr_names.as_ref(),
                attr_values.as_ref(),
            )?;

            make_json_response(
                json!({
                    "Items": output.items,
                    "Count": output.count,
                    "ScannedCount": output.scanned_count
                }),
                StatusCode::OK,
            )
        }

        "BatchGetItem" => {
            let request_items = json_val
                .get("RequestItems")
                .and_then(|v| v.as_object())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "RequestItems is required.",
                    )
                })?;

            let mut responses = serde_json::Map::new();

            for (t_name, spec) in request_items {
                if let Some(keys_arr) = spec.get("Keys").and_then(|v| v.as_array()) {
                    let mut found_items = Vec::new();
                    for k_val in keys_arr {
                        if let Ok(key) =
                            serde_json::from_value::<HashMap<String, AttributeValue>>(k_val.clone())
                        {
                            if let Ok(Some(item)) = engine.get_item(t_name, &key, None, None) {
                                found_items.push(item);
                            }
                        }
                    }
                    responses.insert(t_name.clone(), json!(found_items));
                }
            }

            make_json_response(
                json!({
                    "Responses": Value::Object(responses),
                    "UnprocessedKeys": {}
                }),
                StatusCode::OK,
            )
        }

        "BatchWriteItem" => {
            let request_items = json_val
                .get("RequestItems")
                .and_then(|v| v.as_object())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "RequestItems is required.",
                    )
                })?;

            for (t_name, reqs) in request_items {
                if let Some(arr) = reqs.as_array() {
                    for req in arr {
                        if let Some(put_req) = req.get("PutRequest") {
                            if let Some(item_val) = put_req.get("Item") {
                                if let Ok(item) = serde_json::from_value::<
                                    HashMap<String, AttributeValue>,
                                >(item_val.clone())
                                {
                                    let _ = engine.put_item(t_name, item, None, None, None);
                                }
                            }
                        } else if let Some(del_req) = req.get("DeleteRequest") {
                            if let Some(key_val) = del_req.get("Key") {
                                if let Ok(key) = serde_json::from_value::<
                                    HashMap<String, AttributeValue>,
                                >(key_val.clone())
                                {
                                    let _ = engine.delete_item(t_name, &key, None, None, None);
                                }
                            }
                        }
                    }
                }
            }

            make_json_response(
                json!({
                    "UnprocessedItems": {},
                    "ItemCollectionMetrics": {}
                }),
                StatusCode::OK,
            )
        }

        "ListStreams" => {
            let table_name = json_val.get("TableName").and_then(|v| v.as_str());
            let streams = engine.list_streams(table_name)?;
            make_json_response(json!({ "Streams": streams }), StatusCode::OK)
        }

        "DescribeStream" => {
            let stream_arn = json_val
                .get("StreamArn")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "StreamArn is required.",
                    )
                })?;
            let desc = engine.describe_stream(stream_arn)?;
            make_json_response(json!({ "StreamDescription": desc }), StatusCode::OK)
        }

        "GetShardIterator" => {
            let stream_arn = json_val
                .get("StreamArn")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "StreamArn is required.",
                    )
                })?;
            let shard_id = json_val
                .get("ShardId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "ShardId is required.",
                    )
                })?;
            let iterator_type = json_val
                .get("ShardIteratorType")
                .and_then(|v| v.as_str())
                .unwrap_or("TRIM_HORIZON");
            let seq = json_val.get("SequenceNumber").and_then(|v| v.as_str());

            let iter = engine.get_shard_iterator(stream_arn, shard_id, iterator_type, seq)?;
            make_json_response(json!({ "ShardIterator": iter }), StatusCode::OK)
        }

        "GetRecords" => {
            let shard_iterator = json_val
                .get("ShardIterator")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::dynamodb_bad_request(
                        "ValidationException",
                        "ShardIterator is required.",
                    )
                })?;
            let limit = json_val
                .get("Limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            let (records, next_iter) = engine.get_records(shard_iterator, limit)?;
            let mut res = json!({ "Records": records });
            if let Some(next) = next_iter {
                res["NextShardIterator"] = json!(next);
            }
            make_json_response(res, StatusCode::OK)
        }

        _ => Err(RustStackError::dynamodb_bad_request(
            "InvalidAction",
            format!("Action {} is not supported by DynamoDB.", action),
        )),
    }
}
