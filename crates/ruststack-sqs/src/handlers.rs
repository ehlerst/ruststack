use crate::codec;
use crate::queue::SqsEngine;
use crate::types::{DeleteMessageBatchEntry, MessageAttributeValue, SendMessageBatchEntry};
use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Method, Request, Response, StatusCode, Uri};
use http_body_util::BodyExt;
use ruststack_core::RustStackError;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn handle_sqs_request(engine: Arc<SqsEngine>, req: Request<Body>) -> Response<Body> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let (parts, body) = req.into_parts();
    let method = parts.method;
    let uri = parts.uri;
    let headers = parts.headers;

    let is_json_proto = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .map(|t| t.starts_with("AmazonSQS"))
        .unwrap_or(false)
        || headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|t| t.contains("application/x-amz-json-1.0"))
            .unwrap_or(false);

    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return make_sqs_error_response(
                &RustStackError::BadRequest(e.to_string()),
                &request_id,
                is_json_proto,
            );
        }
    };

    let result = if is_json_proto {
        handle_sqs_json(
            engine.as_ref(),
            &method,
            &uri,
            &headers,
            &body_bytes,
            &request_id,
        )
        .await
    } else {
        handle_sqs_query(
            engine.as_ref(),
            &method,
            &uri,
            &headers,
            &body_bytes,
            &request_id,
        )
        .await
    };

    match result {
        Ok(res) => res,
        Err(err) => make_sqs_error_response(&err, &request_id, is_json_proto),
    }
}

async fn handle_sqs_json(
    engine: &SqsEngine,
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
    let action = target.strip_prefix("AmazonSQS.").unwrap_or(target);

    let json_val: Value = if body.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_slice(body).map_err(|e| RustStackError::BadRequest(e.to_string()))?
    };

    match action {
        "CreateQueue" => {
            let name = json_val
                .get("QueueName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter QueueName.",
                    )
                })?;
            let mut attrs = HashMap::new();
            if let Some(obj) = json_val.get("Attributes").and_then(|v| v.as_object()) {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        attrs.insert(k.clone(), s.to_string());
                    }
                }
            }

            let queue_url = engine.create_queue(name, Some(attrs))?;
            let resp_json = codec::json_create_queue_response(&queue_url);
            make_json_response(resp_json, StatusCode::OK)
        }

        "GetQueueUrl" => {
            let name = json_val
                .get("QueueName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter QueueName.",
                    )
                })?;
            let queue_url = engine.get_queue_url(name)?;
            let resp_json = codec::json_get_queue_url_response(&queue_url);
            make_json_response(resp_json, StatusCode::OK)
        }

        "ListQueues" => {
            let prefix = json_val.get("QueueNamePrefix").and_then(|v| v.as_str());
            let list = engine.list_queues(prefix)?;
            let resp_json = codec::json_list_queues_response(&list);
            make_json_response(resp_json, StatusCode::OK)
        }

        "GetQueueAttributes" => {
            let queue_url = json_val
                .get("QueueUrl")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter QueueUrl.",
                    )
                })?;
            let mut attr_names = Vec::new();
            if let Some(arr) = json_val.get("AttributeNames").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        attr_names.push(s.to_string());
                    }
                }
            }
            let attrs = engine.get_queue_attributes(queue_url, &attr_names)?;
            let resp_json = codec::json_get_queue_attributes_response(&attrs);
            make_json_response(resp_json, StatusCode::OK)
        }

        "SetQueueAttributes" => {
            let queue_url = json_val
                .get("QueueUrl")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter QueueUrl.",
                    )
                })?;
            let mut attrs = HashMap::new();
            if let Some(obj) = json_val.get("Attributes").and_then(|v| v.as_object()) {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        attrs.insert(k.clone(), s.to_string());
                    }
                }
            }
            engine.set_queue_attributes(queue_url, attrs)?;
            make_json_response(serde_json::json!({}), StatusCode::OK)
        }

        "DeleteQueue" => {
            let queue_url = json_val
                .get("QueueUrl")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter QueueUrl.",
                    )
                })?;
            engine.delete_queue(queue_url)?;
            make_json_response(serde_json::json!({}), StatusCode::OK)
        }

        "PurgeQueue" => {
            let queue_url = json_val
                .get("QueueUrl")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter QueueUrl.",
                    )
                })?;
            engine.purge_queue(queue_url)?;
            make_json_response(serde_json::json!({}), StatusCode::OK)
        }

        "SendMessage" => {
            let queue_url = json_val
                .get("QueueUrl")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter QueueUrl.",
                    )
                })?;
            let body = json_val
                .get("MessageBody")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter MessageBody.",
                    )
                })?
                .to_string();
            let delay_seconds = json_val
                .get("DelaySeconds")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let msg_group_id = json_val
                .get("MessageGroupId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let msg_dedup_id = json_val
                .get("MessageDeduplicationId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut msg_attrs = HashMap::new();
            if let Some(obj) = json_val
                .get("MessageAttributes")
                .and_then(|v| v.as_object())
            {
                for (k, v) in obj {
                    if let Some(data_type) = v.get("DataType").and_then(|d| d.as_str()) {
                        let str_val = v
                            .get("StringValue")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());
                        let bin_val = v
                            .get("BinaryValue")
                            .and_then(|b| b.as_str())
                            .map(|s| s.to_string());
                        msg_attrs.insert(
                            k.clone(),
                            MessageAttributeValue {
                                data_type: data_type.to_string(),
                                string_value: str_val,
                                binary_value: bin_val,
                            },
                        );
                    }
                }
            }

            let (msg_id, md5, seq) = engine.send_message(
                queue_url,
                body,
                delay_seconds,
                Some(msg_attrs),
                msg_group_id,
                msg_dedup_id,
            )?;
            let resp_json = codec::json_send_message_response(&msg_id, &md5, seq.as_deref());
            make_json_response(resp_json, StatusCode::OK)
        }

        "SendMessageBatch" => {
            let queue_url = json_val
                .get("QueueUrl")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter QueueUrl.",
                    )
                })?;
            let mut entries = Vec::new();
            if let Some(arr) = json_val.get("Entries").and_then(|v| v.as_array()) {
                for item in arr {
                    let id = item
                        .get("Id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let msg_body = item
                        .get("MessageBody")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let delay = item
                        .get("DelaySeconds")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32);
                    let group = item
                        .get("MessageGroupId")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let dedup = item
                        .get("MessageDeduplicationId")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    entries.push(SendMessageBatchEntry {
                        id,
                        message_body: msg_body,
                        delay_seconds: delay,
                        message_attributes: None,
                        message_group_id: group,
                        message_deduplication_id: dedup,
                    });
                }
            }

            let (successful, failed) = engine.send_message_batch(queue_url, entries)?;
            let resp_json = codec::json_send_message_batch_response(&successful, &failed);
            make_json_response(resp_json, StatusCode::OK)
        }

        "ReceiveMessage" => {
            let queue_url = json_val
                .get("QueueUrl")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter QueueUrl.",
                    )
                })?;
            let max_msgs = json_val
                .get("MaxNumberOfMessages")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u32;
            let vt = json_val
                .get("VisibilityTimeout")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let wait = json_val
                .get("WaitTimeSeconds")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);

            let messages = engine
                .receive_message(queue_url, max_msgs, vt, wait)
                .await?;
            let resp_json = codec::json_receive_message_response(&messages);
            make_json_response(resp_json, StatusCode::OK)
        }

        "DeleteMessage" => {
            let queue_url = json_val
                .get("QueueUrl")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter QueueUrl.",
                    )
                })?;
            let receipt = json_val
                .get("ReceiptHandle")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter ReceiptHandle.",
                    )
                })?;
            engine.delete_message(queue_url, receipt)?;
            make_json_response(serde_json::json!({}), StatusCode::OK)
        }

        "DeleteMessageBatch" => {
            let queue_url = json_val
                .get("QueueUrl")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter QueueUrl.",
                    )
                })?;
            let mut entries = Vec::new();
            if let Some(arr) = json_val.get("Entries").and_then(|v| v.as_array()) {
                for item in arr {
                    let id = item
                        .get("Id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let handle = item
                        .get("ReceiptHandle")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    entries.push(DeleteMessageBatchEntry {
                        id,
                        receipt_handle: handle,
                    });
                }
            }

            let (successful, failed) = engine.delete_message_batch(queue_url, entries)?;
            let resp_json = codec::json_delete_message_batch_response(&successful, &failed);
            make_json_response(resp_json, StatusCode::OK)
        }

        "ChangeMessageVisibility" => {
            let queue_url = json_val
                .get("QueueUrl")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter QueueUrl.",
                    )
                })?;
            let receipt = json_val
                .get("ReceiptHandle")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter ReceiptHandle.",
                    )
                })?;
            let vt = json_val
                .get("VisibilityTimeout")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter VisibilityTimeout.",
                    )
                })? as u32;
            engine.change_message_visibility(queue_url, receipt, vt)?;
            make_json_response(serde_json::json!({}), StatusCode::OK)
        }

        _ => Err(RustStackError::BadRequest(format!(
            "Unsupported SQS Action: {}",
            action
        ))),
    }
}

async fn handle_sqs_query(
    engine: &SqsEngine,
    _method: &Method,
    uri: &Uri,
    _headers: &HeaderMap,
    body: &[u8],
    request_id: &str,
) -> Result<Response<Body>, RustStackError> {
    let mut params = HashMap::new();

    // Parse query string
    if let Some(q) = uri.query() {
        for (k, v) in form_urlencoded::parse(q.as_bytes()) {
            params.insert(k.into_owned(), v.into_owned());
        }
    }

    // Parse form-urlencoded body
    if !body.is_empty() {
        for (k, v) in form_urlencoded::parse(body) {
            params.insert(k.into_owned(), v.into_owned());
        }
    }

    // Extract path-based queue if present in URI
    let path = uri.path();
    let segments: Vec<&str> = path
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let path_queue_url =
        if !segments.is_empty() && (segments[0].len() == 12 || segments[0] == "queue") {
            Some(format!("http://localhost:4566/{}", segments.join("/")))
        } else {
            None
        };

    let action = params.get("Action").map(|s| s.as_str()).unwrap_or("");
    let queue_url = params
        .get("QueueUrl")
        .cloned()
        .or(path_queue_url)
        .unwrap_or_default();

    match action {
        "CreateQueue" => {
            let name = params.get("QueueName").ok_or_else(|| {
                RustStackError::sqs_bad_request(
                    "MissingParameter",
                    "The request must contain the parameter QueueName.",
                )
            })?;
            let mut attrs = HashMap::new();
            for i in 1..=20 {
                let name_key = format!("Attribute.{}.Name", i);
                let val_key = format!("Attribute.{}.Value", i);
                if let (Some(k), Some(v)) = (params.get(&name_key), params.get(&val_key)) {
                    attrs.insert(k.clone(), v.clone());
                }
            }

            let created_url = engine.create_queue(name, Some(attrs))?;
            let xml_body = codec::xml_create_queue_response(&created_url, request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        "GetQueueUrl" => {
            let name = params.get("QueueName").ok_or_else(|| {
                RustStackError::sqs_bad_request(
                    "MissingParameter",
                    "The request must contain the parameter QueueName.",
                )
            })?;
            let q_url = engine.get_queue_url(name)?;
            let xml_body = codec::xml_get_queue_url_response(&q_url, request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        "ListQueues" => {
            let prefix = params.get("QueueNamePrefix").map(|s| s.as_str());
            let list = engine.list_queues(prefix)?;
            let xml_body = codec::xml_list_queues_response(&list, request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        "GetQueueAttributes" => {
            let mut attr_names = Vec::new();
            for i in 1..=20 {
                let key = format!("AttributeName.{}", i);
                if let Some(v) = params.get(&key) {
                    attr_names.push(v.clone());
                }
            }
            if let Some(single) = params.get("AttributeName") {
                attr_names.push(single.clone());
            }

            let attrs = engine.get_queue_attributes(&queue_url, &attr_names)?;
            let xml_body = codec::xml_get_queue_attributes_response(&attrs, request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        "SetQueueAttributes" => {
            let mut attrs = HashMap::new();
            for i in 1..=20 {
                let name_key = format!("Attribute.{}.Name", i);
                let val_key = format!("Attribute.{}.Value", i);
                if let (Some(k), Some(v)) = (params.get(&name_key), params.get(&val_key)) {
                    attrs.insert(k.clone(), v.clone());
                }
            }

            engine.set_queue_attributes(&queue_url, attrs)?;
            let xml_body = codec::xml_empty_response("SetQueueAttributes", request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        "DeleteQueue" => {
            engine.delete_queue(&queue_url)?;
            let xml_body = codec::xml_empty_response("DeleteQueue", request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        "PurgeQueue" => {
            engine.purge_queue(&queue_url)?;
            let xml_body = codec::xml_empty_response("PurgeQueue", request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        "SendMessage" => {
            let body_str = params
                .get("MessageBody")
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter MessageBody.",
                    )
                })?
                .clone();
            let delay_seconds = params.get("DelaySeconds").and_then(|v| v.parse().ok());
            let msg_group_id = params.get("MessageGroupId").cloned();
            let msg_dedup_id = params.get("MessageDeduplicationId").cloned();

            let mut msg_attrs = HashMap::new();
            for i in 1..=10 {
                let name_key = format!("MessageAttribute.{}.Name", i);
                let dt_key = format!("MessageAttribute.{}.Value.DataType", i);
                let str_key = format!("MessageAttribute.{}.Value.StringValue", i);
                if let (Some(k), Some(dt)) = (params.get(&name_key), params.get(&dt_key)) {
                    let str_val = params.get(&str_key).cloned();
                    msg_attrs.insert(
                        k.clone(),
                        MessageAttributeValue {
                            data_type: dt.clone(),
                            string_value: str_val,
                            binary_value: None,
                        },
                    );
                }
            }

            let (msg_id, md5, seq) = engine.send_message(
                &queue_url,
                body_str,
                delay_seconds,
                Some(msg_attrs),
                msg_group_id,
                msg_dedup_id,
            )?;

            let xml_body =
                codec::xml_send_message_response(&msg_id, &md5, seq.as_deref(), request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        "SendMessageBatch" => {
            let mut entries = Vec::new();
            for i in 1..=10 {
                let id_key = format!("SendMessageBatchRequestEntry.{}.Id", i);
                let body_key = format!("SendMessageBatchRequestEntry.{}.MessageBody", i);
                let delay_key = format!("SendMessageBatchRequestEntry.{}.DelaySeconds", i);
                let group_key = format!("SendMessageBatchRequestEntry.{}.MessageGroupId", i);
                let dedup_key =
                    format!("SendMessageBatchRequestEntry.{}.MessageDeduplicationId", i);

                if let (Some(id), Some(msg_body)) = (params.get(&id_key), params.get(&body_key)) {
                    let delay = params.get(&delay_key).and_then(|v| v.parse().ok());
                    let group = params.get(&group_key).cloned();
                    let dedup = params.get(&dedup_key).cloned();

                    entries.push(SendMessageBatchEntry {
                        id: id.clone(),
                        message_body: msg_body.clone(),
                        delay_seconds: delay,
                        message_attributes: None,
                        message_group_id: group,
                        message_deduplication_id: dedup,
                    });
                }
            }

            let (successful, failed) = engine.send_message_batch(&queue_url, entries)?;
            let xml_body = codec::xml_send_message_batch_response(&successful, &failed, request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        "ReceiveMessage" => {
            let max_msgs: u32 = params
                .get("MaxNumberOfMessages")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);
            let vt: Option<u32> = params.get("VisibilityTimeout").and_then(|v| v.parse().ok());
            let wait: Option<u32> = params.get("WaitTimeSeconds").and_then(|v| v.parse().ok());

            let messages = engine
                .receive_message(&queue_url, max_msgs, vt, wait)
                .await?;
            let xml_body = codec::xml_receive_message_response(&messages, request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        "DeleteMessage" => {
            let receipt = params.get("ReceiptHandle").ok_or_else(|| {
                RustStackError::sqs_bad_request(
                    "MissingParameter",
                    "The request must contain the parameter ReceiptHandle.",
                )
            })?;
            engine.delete_message(&queue_url, receipt)?;
            let xml_body = codec::xml_empty_response("DeleteMessage", request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        "DeleteMessageBatch" => {
            let mut entries = Vec::new();
            for i in 1..=10 {
                let id_key = format!("DeleteMessageBatchRequestEntry.{}.Id", i);
                let handle_key = format!("DeleteMessageBatchRequestEntry.{}.ReceiptHandle", i);
                if let (Some(id), Some(handle)) = (params.get(&id_key), params.get(&handle_key)) {
                    entries.push(DeleteMessageBatchEntry {
                        id: id.clone(),
                        receipt_handle: handle.clone(),
                    });
                }
            }

            let (successful, failed) = engine.delete_message_batch(&queue_url, entries)?;
            let xml_body =
                codec::xml_delete_message_batch_response(&successful, &failed, request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        "ChangeMessageVisibility" => {
            let receipt = params.get("ReceiptHandle").ok_or_else(|| {
                RustStackError::sqs_bad_request(
                    "MissingParameter",
                    "The request must contain the parameter ReceiptHandle.",
                )
            })?;
            let vt: u32 = params
                .get("VisibilityTimeout")
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| {
                    RustStackError::sqs_bad_request(
                        "MissingParameter",
                        "The request must contain the parameter VisibilityTimeout.",
                    )
                })?;

            engine.change_message_visibility(&queue_url, receipt, vt)?;
            let xml_body = codec::xml_empty_response("ChangeMessageVisibility", request_id);
            make_xml_response(xml_body, StatusCode::OK, request_id)
        }

        _ => Err(RustStackError::BadRequest(format!(
            "Unsupported SQS Query Action: {}",
            action
        ))),
    }
}

fn make_xml_response(
    xml_body: String,
    status: StatusCode,
    _request_id: &str,
) -> Result<Response<Body>, RustStackError> {
    let mut res = Response::new(Body::from(xml_body));
    *res.status_mut() = status;
    res.headers_mut()
        .insert("content-type", HeaderValue::from_static("text/xml"));
    Ok(res)
}

fn make_json_response(
    json_val: Value,
    status: StatusCode,
) -> Result<Response<Body>, RustStackError> {
    let body_str = json_val.to_string();
    let mut res = Response::new(Body::from(body_str));
    *res.status_mut() = status;
    res.headers_mut().insert(
        "content-type",
        HeaderValue::from_static("application/x-amz-json-1.0"),
    );
    Ok(res)
}

pub fn make_sqs_error_response(
    err: &RustStackError,
    request_id: &str,
    is_json: bool,
) -> Response<Body> {
    if is_json {
        let json_err = err.to_sqs_json();
        let mut res = Response::new(Body::from(json_err.to_string()));
        *res.status_mut() = err.status_code();
        res.headers_mut().insert(
            "content-type",
            HeaderValue::from_static("application/x-amz-json-1.0"),
        );
        res
    } else {
        let xml_err = err.to_sqs_xml(request_id);
        let mut res = Response::new(Body::from(xml_err));
        *res.status_mut() = err.status_code();
        res.headers_mut()
            .insert("content-type", HeaderValue::from_static("text/xml"));
        res
    }
}
