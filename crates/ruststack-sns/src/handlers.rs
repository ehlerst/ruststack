use crate::codec;
use crate::topic::SnsEngine;
use crate::types::{MessageAttributeValue, PublishBatchEntry};
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use http_body_util::BodyExt;
use ruststack_core::RustStackError;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn handle_sns_request(engine: Arc<SnsEngine>, req: Request<Body>) -> Response<Body> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let (parts, body) = req.into_parts();
    let method = parts.method;
    let uri = parts.uri;
    let headers = parts.headers;

    let is_json_proto = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .map(|t| t.starts_with("AmazonSNS"))
        .unwrap_or(false)
        || headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|t| t.contains("application/x-amz-json-1.0"))
            .unwrap_or(false);

    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return make_sns_error_response(
                &RustStackError::BadRequest(e.to_string()),
                &request_id,
                is_json_proto,
            );
        }
    };

    let result = if is_json_proto {
        handle_sns_json(
            engine.as_ref(),
            &method,
            &uri,
            &headers,
            &body_bytes,
            &request_id,
        )
        .await
    } else {
        handle_sns_query(
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
        Err(err) => make_sns_error_response(&err, &request_id, is_json_proto),
    }
}

pub fn make_sns_error_response(
    err: &RustStackError,
    request_id: &str,
    is_json_proto: bool,
) -> Response<Body> {
    let status = err.status_code();
    if is_json_proto {
        let json_err = err.to_sns_json();
        Response::builder()
            .status(status)
            .header("content-type", "application/x-amz-json-1.0")
            .header("x-amzn-requestid", request_id)
            .body(Body::from(json_err.to_string()))
            .unwrap()
    } else {
        let xml_err = err.to_sns_xml(request_id);
        Response::builder()
            .status(status)
            .header("content-type", "text/xml;charset=UTF-8")
            .header("x-amzn-requestid", request_id)
            .body(Body::from(xml_err))
            .unwrap()
    }
}

fn make_xml_response(
    xml: String,
    status: StatusCode,
    request_id: &str,
) -> Result<Response<Body>, RustStackError> {
    Response::builder()
        .status(status)
        .header("content-type", "text/xml;charset=UTF-8")
        .header("x-amzn-requestid", request_id)
        .body(Body::from(xml))
        .map_err(|e| RustStackError::Internal(e.to_string()))
}

fn make_json_response(val: Value, status: StatusCode) -> Result<Response<Body>, RustStackError> {
    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(val.to_string()))
        .map_err(|e| RustStackError::Internal(e.to_string()))
}

// --- JSON Protocol Dispatcher ---

async fn handle_sns_json(
    engine: &SnsEngine,
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
    let action = target.strip_prefix("AmazonSNS.").unwrap_or(target);

    let json_val: Value = if body.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_slice(body).map_err(|e| RustStackError::BadRequest(e.to_string()))?
    };

    match action {
        "CreateTopic" => {
            let name = json_val
                .get("Name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter Name.",
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
            let topic_arn = engine.create_topic(name, Some(attrs))?;
            let resp = codec::json_create_topic_response(&topic_arn);
            make_json_response(resp, StatusCode::OK)
        }

        "DeleteTopic" => {
            let topic_arn = json_val
                .get("TopicArn")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter TopicArn.",
                    )
                })?;
            engine.delete_topic(topic_arn)?;
            make_json_response(serde_json::json!({}), StatusCode::OK)
        }

        "ListTopics" => {
            let list = engine.list_topics()?;
            let resp = codec::json_list_topics_response(&list);
            make_json_response(resp, StatusCode::OK)
        }

        "GetTopicAttributes" => {
            let topic_arn = json_val
                .get("TopicArn")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter TopicArn.",
                    )
                })?;
            let attrs = engine.get_topic_attributes(topic_arn)?;
            let resp = codec::json_get_topic_attributes_response(&attrs);
            make_json_response(resp, StatusCode::OK)
        }

        "SetTopicAttributes" => {
            let topic_arn = json_val
                .get("TopicArn")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter TopicArn.",
                    )
                })?;
            let attr_name = json_val
                .get("AttributeName")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let attr_value = json_val
                .get("AttributeValue")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            engine.set_topic_attributes(topic_arn, attr_name, attr_value)?;
            make_json_response(serde_json::json!({}), StatusCode::OK)
        }

        "Subscribe" => {
            let topic_arn = json_val
                .get("TopicArn")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter TopicArn.",
                    )
                })?;
            let protocol = json_val
                .get("Protocol")
                .and_then(|v| v.as_str())
                .unwrap_or("sqs");
            let endpoint = json_val
                .get("Endpoint")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let mut attrs = HashMap::new();
            if let Some(obj) = json_val.get("Attributes").and_then(|v| v.as_object()) {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        attrs.insert(k.clone(), s.to_string());
                    }
                }
            }

            let sub_arn = engine.subscribe(topic_arn, protocol, endpoint, Some(attrs))?;
            let resp = codec::json_subscribe_response(&sub_arn);
            make_json_response(resp, StatusCode::OK)
        }

        "Unsubscribe" => {
            let sub_arn = json_val
                .get("SubscriptionArn")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter SubscriptionArn.",
                    )
                })?;
            engine.unsubscribe(sub_arn)?;
            make_json_response(serde_json::json!({}), StatusCode::OK)
        }

        "ListSubscriptions" => {
            let list = engine.list_subscriptions()?;
            let resp = codec::json_list_subscriptions_response(&list);
            make_json_response(resp, StatusCode::OK)
        }

        "ListSubscriptionsByTopic" => {
            let topic_arn = json_val
                .get("TopicArn")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter TopicArn.",
                    )
                })?;
            let list = engine.list_subscriptions_by_topic(topic_arn)?;
            let resp = codec::json_list_subscriptions_response(&list);
            make_json_response(resp, StatusCode::OK)
        }

        "GetSubscriptionAttributes" => {
            let sub_arn = json_val
                .get("SubscriptionArn")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter SubscriptionArn.",
                    )
                })?;
            let attrs = engine.get_subscription_attributes(sub_arn)?;
            let resp = codec::json_get_subscription_attributes_response(&attrs);
            make_json_response(resp, StatusCode::OK)
        }

        "SetSubscriptionAttributes" => {
            let sub_arn = json_val
                .get("SubscriptionArn")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter SubscriptionArn.",
                    )
                })?;
            let attr_name = json_val
                .get("AttributeName")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let attr_value = json_val
                .get("AttributeValue")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            engine.set_subscription_attributes(sub_arn, attr_name, attr_value)?;
            make_json_response(serde_json::json!({}), StatusCode::OK)
        }

        "Publish" => {
            let topic_arn = json_val
                .get("TopicArn")
                .and_then(|v| v.as_str())
                .or_else(|| json_val.get("TargetArn").and_then(|v| v.as_str()))
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter TopicArn.",
                    )
                })?;
            let message = json_val
                .get("Message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter Message.",
                    )
                })?
                .to_string();
            let subject = json_val
                .get("Subject")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let dedup_id = json_val
                .get("MessageDeduplicationId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let group_id = json_val
                .get("MessageGroupId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut msg_attrs = HashMap::new();
            if let Some(obj) = json_val
                .get("MessageAttributes")
                .and_then(|v| v.as_object())
            {
                for (k, v) in obj {
                    let dt = v
                        .get("DataType")
                        .and_then(|d| d.as_str())
                        .unwrap_or("String");
                    let sv = v
                        .get("StringValue")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());
                    let bv = v
                        .get("BinaryValue")
                        .and_then(|b| b.as_str())
                        .map(|s| s.to_string());
                    msg_attrs.insert(
                        k.clone(),
                        MessageAttributeValue {
                            data_type: dt.to_string(),
                            string_value: sv,
                            binary_value: bv,
                        },
                    );
                }
            }

            let (msg_id, seq) = engine.publish(
                topic_arn,
                message,
                subject,
                if msg_attrs.is_empty() {
                    None
                } else {
                    Some(msg_attrs)
                },
                dedup_id,
                group_id,
            )?;
            let resp = codec::json_publish_response(&msg_id, seq.as_deref());
            make_json_response(resp, StatusCode::OK)
        }

        "PublishBatch" => {
            let topic_arn = json_val
                .get("TopicArn")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter TopicArn.",
                    )
                })?;
            let mut entries = Vec::new();
            if let Some(arr) = json_val
                .get("PublishBatchRequestEntries")
                .and_then(|v| v.as_array())
            {
                for item in arr {
                    let id = item
                        .get("Id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let msg = item
                        .get("Message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let subject = item
                        .get("Subject")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    entries.push(PublishBatchEntry {
                        id,
                        message: msg,
                        subject,
                        message_attributes: None,
                        message_deduplication_id: None,
                        message_group_id: None,
                    });
                }
            }
            let (succ, fail) = engine.publish_batch(topic_arn, entries)?;
            let resp = codec::json_publish_batch_response(&succ, &fail);
            make_json_response(resp, StatusCode::OK)
        }

        _ => Err(RustStackError::sns_bad_request(
            "InvalidAction",
            format!("The action {} is not valid for this endpoint.", action),
        )),
    }
}

// --- Query Protocol Dispatcher ---

async fn handle_sns_query(
    engine: &SnsEngine,
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

    let action = params.get("Action").map(|s| s.as_str()).unwrap_or("");

    match action {
        "CreateTopic" => {
            let name = params.get("Name").ok_or_else(|| {
                RustStackError::sns_bad_request(
                    "InvalidParameter",
                    "The request must contain the parameter Name.",
                )
            })?;
            let topic_arn = engine.create_topic(name, None)?;
            let xml = codec::xml_create_topic_response(&topic_arn, request_id);
            make_xml_response(xml, StatusCode::OK, request_id)
        }

        "DeleteTopic" => {
            let topic_arn = params.get("TopicArn").ok_or_else(|| {
                RustStackError::sns_bad_request(
                    "InvalidParameter",
                    "The request must contain the parameter TopicArn.",
                )
            })?;
            engine.delete_topic(topic_arn)?;
            let xml = codec::xml_delete_topic_response(request_id);
            make_xml_response(xml, StatusCode::OK, request_id)
        }

        "ListTopics" => {
            let list = engine.list_topics()?;
            let xml = codec::xml_list_topics_response(&list, request_id);
            make_xml_response(xml, StatusCode::OK, request_id)
        }

        "GetTopicAttributes" => {
            let topic_arn = params.get("TopicArn").ok_or_else(|| {
                RustStackError::sns_bad_request(
                    "InvalidParameter",
                    "The request must contain the parameter TopicArn.",
                )
            })?;
            let attrs = engine.get_topic_attributes(topic_arn)?;
            let xml = codec::xml_get_topic_attributes_response(&attrs, request_id);
            make_xml_response(xml, StatusCode::OK, request_id)
        }

        "SetTopicAttributes" => {
            let topic_arn = params.get("TopicArn").ok_or_else(|| {
                RustStackError::sns_bad_request(
                    "InvalidParameter",
                    "The request must contain the parameter TopicArn.",
                )
            })?;
            let attr_name = params
                .get("AttributeName")
                .map(|s| s.as_str())
                .unwrap_or("");
            let attr_value = params
                .get("AttributeValue")
                .map(|s| s.as_str())
                .unwrap_or("");
            engine.set_topic_attributes(topic_arn, attr_name, attr_value)?;
            let xml = codec::xml_set_topic_attributes_response(request_id);
            make_xml_response(xml, StatusCode::OK, request_id)
        }

        "Subscribe" => {
            let topic_arn = params.get("TopicArn").ok_or_else(|| {
                RustStackError::sns_bad_request(
                    "InvalidParameter",
                    "The request must contain the parameter TopicArn.",
                )
            })?;
            let protocol = params.get("Protocol").map(|s| s.as_str()).unwrap_or("sqs");
            let endpoint = params.get("Endpoint").map(|s| s.as_str()).unwrap_or("");
            let sub_arn = engine.subscribe(topic_arn, protocol, endpoint, None)?;
            let xml = codec::xml_subscribe_response(&sub_arn, request_id);
            make_xml_response(xml, StatusCode::OK, request_id)
        }

        "Unsubscribe" => {
            let sub_arn = params.get("SubscriptionArn").ok_or_else(|| {
                RustStackError::sns_bad_request(
                    "InvalidParameter",
                    "The request must contain the parameter SubscriptionArn.",
                )
            })?;
            engine.unsubscribe(sub_arn)?;
            let xml = codec::xml_unsubscribe_response(request_id);
            make_xml_response(xml, StatusCode::OK, request_id)
        }

        "ListSubscriptions" => {
            let list = engine.list_subscriptions()?;
            let xml = codec::xml_list_subscriptions_response(&list, request_id);
            make_xml_response(xml, StatusCode::OK, request_id)
        }

        "ListSubscriptionsByTopic" => {
            let topic_arn = params.get("TopicArn").ok_or_else(|| {
                RustStackError::sns_bad_request(
                    "InvalidParameter",
                    "The request must contain the parameter TopicArn.",
                )
            })?;
            let list = engine.list_subscriptions_by_topic(topic_arn)?;
            let xml = codec::xml_list_subscriptions_response(&list, request_id);
            make_xml_response(xml, StatusCode::OK, request_id)
        }

        "GetSubscriptionAttributes" => {
            let sub_arn = params.get("SubscriptionArn").ok_or_else(|| {
                RustStackError::sns_bad_request(
                    "InvalidParameter",
                    "The request must contain the parameter SubscriptionArn.",
                )
            })?;
            let attrs = engine.get_subscription_attributes(sub_arn)?;
            let xml = codec::xml_get_subscription_attributes_response(&attrs, request_id);
            make_xml_response(xml, StatusCode::OK, request_id)
        }

        "SetSubscriptionAttributes" => {
            let sub_arn = params.get("SubscriptionArn").ok_or_else(|| {
                RustStackError::sns_bad_request(
                    "InvalidParameter",
                    "The request must contain the parameter SubscriptionArn.",
                )
            })?;
            let attr_name = params
                .get("AttributeName")
                .map(|s| s.as_str())
                .unwrap_or("");
            let attr_value = params
                .get("AttributeValue")
                .map(|s| s.as_str())
                .unwrap_or("");
            engine.set_subscription_attributes(sub_arn, attr_name, attr_value)?;
            let xml = codec::xml_set_subscription_attributes_response(request_id);
            make_xml_response(xml, StatusCode::OK, request_id)
        }

        "Publish" => {
            let topic_arn = params
                .get("TopicArn")
                .or_else(|| params.get("TargetArn"))
                .ok_or_else(|| {
                    RustStackError::sns_bad_request(
                        "InvalidParameter",
                        "The request must contain the parameter TopicArn.",
                    )
                })?;
            let message = params.get("Message").cloned().ok_or_else(|| {
                RustStackError::sns_bad_request(
                    "InvalidParameter",
                    "The request must contain the parameter Message.",
                )
            })?;
            let subject = params.get("Subject").cloned();
            let dedup_id = params.get("MessageDeduplicationId").cloned();
            let group_id = params.get("MessageGroupId").cloned();

            let (msg_id, seq) =
                engine.publish(topic_arn, message, subject, None, dedup_id, group_id)?;
            let xml = codec::xml_publish_response(&msg_id, seq.as_deref(), request_id);
            make_xml_response(xml, StatusCode::OK, request_id)
        }

        _ => Err(RustStackError::sns_bad_request(
            "InvalidAction",
            format!("The action {} is not valid for this endpoint.", action),
        )),
    }
}
