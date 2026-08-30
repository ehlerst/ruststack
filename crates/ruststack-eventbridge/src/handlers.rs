use crate::bus::EventBridgeEngine;
use crate::types::{PutEventsRequestEntry, Target};
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use http_body_util::BodyExt;
use ruststack_core::RustStackError;
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_eventbridge_request(
    engine: Arc<EventBridgeEngine>,
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
            return make_eventbridge_error_response(
                &RustStackError::BadRequest(e.to_string()),
                &request_id,
            );
        }
    };

    let result = handle_eventbridge_json(
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
        Err(err) => make_eventbridge_error_response(&err, &request_id),
    }
}

pub fn make_eventbridge_error_response(err: &RustStackError, request_id: &str) -> Response<Body> {
    let status = err.status_code();
    let json_err = err.to_eventbridge_json();
    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.1")
        .header("x-amzn-requestid", request_id)
        .body(Body::from(json_err.to_string()))
        .unwrap()
}

fn make_json_response(val: Value, status: StatusCode) -> Result<Response<Body>, RustStackError> {
    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(val.to_string()))
        .map_err(|e| RustStackError::Internal(e.to_string()))
}

async fn handle_eventbridge_json(
    engine: &EventBridgeEngine,
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
        .strip_prefix("AWSEvents.")
        .or_else(|| target.strip_prefix("AmazonEventBridge."))
        .or_else(|| target.strip_prefix("EventBridge."))
        .unwrap_or(target);

    let json_val: Value = if body.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_slice(body).map_err(|e| RustStackError::BadRequest(e.to_string()))?
    };

    match action {
        "PutEvents" => {
            let mut entries = Vec::new();
            if let Some(arr) = json_val.get("Entries").and_then(|v| v.as_array()) {
                for item in arr {
                    let source = item
                        .get("Source")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let detail_type = item
                        .get("DetailType")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let detail = item
                        .get("Detail")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let bus_name = item
                        .get("EventBusName")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let resources = item.get("Resources").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter()
                            .filter_map(|r| r.as_str().map(|s| s.to_string()))
                            .collect()
                    });

                    entries.push(PutEventsRequestEntry {
                        time: None,
                        source,
                        resources,
                        detail_type,
                        detail,
                        event_bus_name: bus_name,
                        trace_header: None,
                    });
                }
            }

            let (failed_count, result_entries) = engine.put_events(entries)?;
            let entries_json: Vec<_> = result_entries
                .into_iter()
                .map(|r| {
                    json!({
                        "EventId": r.event_id,
                        "ErrorCode": r.error_code,
                        "ErrorMessage": r.error_message
                    })
                })
                .collect();

            let resp = json!({
                "FailedEntryCount": failed_count,
                "Entries": entries_json
            });
            make_json_response(resp, StatusCode::OK)
        }

        "CreateEventBus" => {
            let name = json_val
                .get("Name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::eventbridge_bad_request(
                        "InvalidParameterValueException",
                        "Parameter Name is required.",
                    )
                })?;
            let arn = engine.create_event_bus(name)?;
            make_json_response(json!({ "EventBusArn": arn }), StatusCode::OK)
        }

        "DeleteEventBus" => {
            let name = json_val
                .get("Name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::eventbridge_bad_request(
                        "InvalidParameterValueException",
                        "Parameter Name is required.",
                    )
                })?;
            engine.delete_event_bus(name)?;
            make_json_response(json!({}), StatusCode::OK)
        }

        "ListEventBuses" => {
            let prefix = json_val.get("NamePrefix").and_then(|v| v.as_str());
            let list = engine.list_event_buses(prefix)?;
            let buses_json: Vec<_> = list
                .into_iter()
                .map(|b| {
                    json!({
                        "Name": b.name,
                        "Arn": b.arn,
                        "Policy": b.policy
                    })
                })
                .collect();
            make_json_response(json!({ "EventBuses": buses_json }), StatusCode::OK)
        }

        "DescribeEventBus" => {
            let name = json_val.get("Name").and_then(|v| v.as_str());
            let bus = engine.describe_event_bus(name)?;
            make_json_response(
                json!({
                    "Name": bus.name,
                    "Arn": bus.arn,
                    "Policy": bus.policy
                }),
                StatusCode::OK,
            )
        }

        "PutRule" => {
            let name = json_val
                .get("Name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::eventbridge_bad_request(
                        "InvalidParameterValueException",
                        "Parameter Name is required.",
                    )
                })?;
            let bus_name = json_val.get("EventBusName").and_then(|v| v.as_str());
            let event_pattern = json_val
                .get("EventPattern")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let state = json_val.get("State").and_then(|v| v.as_str());
            let desc = json_val
                .get("Description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let sched = json_val
                .get("ScheduleExpression")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let arn = engine.put_rule(name, bus_name, event_pattern, state, desc, sched)?;
            make_json_response(json!({ "RuleArn": arn }), StatusCode::OK)
        }

        "DeleteRule" => {
            let name = json_val
                .get("Name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::eventbridge_bad_request(
                        "InvalidParameterValueException",
                        "Parameter Name is required.",
                    )
                })?;
            let bus_name = json_val.get("EventBusName").and_then(|v| v.as_str());
            let force = json_val
                .get("Force")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            engine.delete_rule(name, bus_name, force)?;
            make_json_response(json!({}), StatusCode::OK)
        }

        "ListRules" => {
            let bus_name = json_val.get("EventBusName").and_then(|v| v.as_str());
            let prefix = json_val.get("NamePrefix").and_then(|v| v.as_str());
            let list = engine.list_rules(bus_name, prefix)?;
            let rules_json: Vec<_> = list
                .into_iter()
                .map(|r| {
                    json!({
                        "Name": r.name,
                        "Arn": r.arn,
                        "EventBusName": r.event_bus_name,
                        "EventPattern": r.event_pattern,
                        "State": r.state,
                        "Description": r.description,
                        "ScheduleExpression": r.schedule_expression
                    })
                })
                .collect();
            make_json_response(json!({ "Rules": rules_json }), StatusCode::OK)
        }

        "DescribeRule" => {
            let name = json_val
                .get("Name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::eventbridge_bad_request(
                        "InvalidParameterValueException",
                        "Parameter Name is required.",
                    )
                })?;
            let bus_name = json_val.get("EventBusName").and_then(|v| v.as_str());
            let r = engine.describe_rule(name, bus_name)?;
            make_json_response(
                json!({
                    "Name": r.name,
                    "Arn": r.arn,
                    "EventBusName": r.event_bus_name,
                    "EventPattern": r.event_pattern,
                    "State": r.state,
                    "Description": r.description,
                    "ScheduleExpression": r.schedule_expression
                }),
                StatusCode::OK,
            )
        }

        "EnableRule" => {
            let name = json_val.get("Name").and_then(|v| v.as_str()).unwrap_or("");
            let bus_name = json_val.get("EventBusName").and_then(|v| v.as_str());
            engine.enable_rule(name, bus_name)?;
            make_json_response(json!({}), StatusCode::OK)
        }

        "DisableRule" => {
            let name = json_val.get("Name").and_then(|v| v.as_str()).unwrap_or("");
            let bus_name = json_val.get("EventBusName").and_then(|v| v.as_str());
            engine.disable_rule(name, bus_name)?;
            make_json_response(json!({}), StatusCode::OK)
        }

        "PutTargets" => {
            let rule_name = json_val
                .get("Rule")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::eventbridge_bad_request(
                        "InvalidParameterValueException",
                        "Parameter Rule is required.",
                    )
                })?;
            let bus_name = json_val.get("EventBusName").and_then(|v| v.as_str());
            let mut targets = Vec::new();

            if let Some(arr) = json_val.get("Targets").and_then(|v| v.as_array()) {
                for item in arr {
                    let id = item
                        .get("Id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let arn = item
                        .get("Arn")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let input = item
                        .get("Input")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let input_path = item
                        .get("InputPath")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let role_arn = item
                        .get("RoleArn")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    targets.push(Target {
                        id,
                        arn,
                        input,
                        input_path,
                        role_arn,
                    });
                }
            }

            let results = engine.put_targets(rule_name, bus_name, targets)?;
            let results_json: Vec<_> = results
                .into_iter()
                .map(|res| {
                    json!({
                        "TargetId": res.target_id,
                        "ErrorCode": res.error_code,
                        "ErrorMessage": res.error_message
                    })
                })
                .collect();

            make_json_response(
                json!({
                    "FailedEntryCount": 0,
                    "FailedEntries": results_json
                }),
                StatusCode::OK,
            )
        }

        "RemoveTargets" => {
            let rule_name = json_val
                .get("Rule")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::eventbridge_bad_request(
                        "InvalidParameterValueException",
                        "Parameter Rule is required.",
                    )
                })?;
            let bus_name = json_val.get("EventBusName").and_then(|v| v.as_str());
            let mut ids = Vec::new();
            if let Some(arr) = json_val.get("Ids").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        ids.push(s.to_string());
                    }
                }
            }

            let results = engine.remove_targets(rule_name, bus_name, ids)?;
            let results_json: Vec<_> = results
                .into_iter()
                .map(|res| {
                    json!({
                        "TargetId": res.target_id,
                        "ErrorCode": res.error_code,
                        "ErrorMessage": res.error_message
                    })
                })
                .collect();

            make_json_response(
                json!({
                    "FailedEntryCount": 0,
                    "FailedEntries": results_json
                }),
                StatusCode::OK,
            )
        }

        "ListTargetsByRule" => {
            let rule_name = json_val
                .get("Rule")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::eventbridge_bad_request(
                        "InvalidParameterValueException",
                        "Parameter Rule is required.",
                    )
                })?;
            let bus_name = json_val.get("EventBusName").and_then(|v| v.as_str());
            let list = engine.list_targets_by_rule(rule_name, bus_name)?;
            let targets_json: Vec<_> = list
                .into_iter()
                .map(|t| {
                    json!({
                        "Id": t.id,
                        "Arn": t.arn,
                        "Input": t.input,
                        "InputPath": t.input_path,
                        "RoleArn": t.role_arn
                    })
                })
                .collect();

            make_json_response(json!({ "Targets": targets_json }), StatusCode::OK)
        }

        _ => Err(RustStackError::eventbridge_bad_request(
            "InvalidAction",
            format!("Action {} is not supported by EventBridge.", action),
        )),
    }
}
