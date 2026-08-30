use crate::store::SsmEngine;
use crate::types::{Parameter, PutParameterRequest};
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use http_body_util::BodyExt;
use ruststack_core::RustStackError;
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_ssm_request(engine: Arc<SsmEngine>, req: Request<Body>) -> Response<Body> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let (parts, body) = req.into_parts();
    let method = parts.method;
    let uri = parts.uri;
    let headers = parts.headers;

    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return make_ssm_error_response(
                &RustStackError::BadRequest(e.to_string()),
                &request_id,
            );
        }
    };

    let result = handle_ssm_json(
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
        Err(err) => make_ssm_error_response(&err, &request_id),
    }
}

pub fn make_ssm_error_response(err: &RustStackError, request_id: &str) -> Response<Body> {
    let status = err.status_code();
    let json_err = err.to_ssm_json();
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

fn format_parameter_json(p: &Parameter) -> Value {
    json!({
        "ARN": p.arn,
        "DataType": p.data_type,
        "LastModifiedDate": p.last_modified_date.timestamp(),
        "Name": p.name,
        "Type": p.parameter_type.as_str(),
        "Value": p.value,
        "Version": p.version,
        "Tier": p.tier
    })
}

async fn handle_ssm_json(
    engine: &SsmEngine,
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

    let action = target.strip_prefix("AmazonSSM.").unwrap_or(target);

    let json_val: Value = if body.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_slice(body).map_err(|e| RustStackError::BadRequest(e.to_string()))?
    };

    match action {
        "PutParameter" => {
            let name = json_val
                .get("Name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::ssm_bad_request(
                        "ValidationException",
                        "Parameter Name is required.",
                    )
                })?
                .to_string();
            let value = json_val
                .get("Value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::ssm_bad_request(
                        "ValidationException",
                        "Parameter Value is required.",
                    )
                })?
                .to_string();
            let param_type = json_val
                .get("Type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let description = json_val
                .get("Description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let overwrite = json_val.get("Overwrite").and_then(|v| v.as_bool());
            let key_id = json_val
                .get("KeyId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let tier = json_val
                .get("Tier")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let data_type = json_val
                .get("DataType")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let allowed_pattern = json_val
                .get("AllowedPattern")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let version = engine.put_parameter(PutParameterRequest {
                name,
                value,
                parameter_type: param_type,
                description,
                overwrite,
                key_id,
                tier,
                data_type,
                allowed_pattern,
            })?;

            make_json_response(
                json!({
                    "Tier": "Standard",
                    "Version": version
                }),
                StatusCode::OK,
            )
        }

        "GetParameter" => {
            let name = json_val
                .get("Name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::ssm_bad_request(
                        "ValidationException",
                        "Parameter Name is required.",
                    )
                })?;
            let with_decryption = json_val
                .get("WithDecryption")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let param = engine.get_parameter(name, with_decryption)?;
            make_json_response(
                json!({
                    "Parameter": format_parameter_json(&param)
                }),
                StatusCode::OK,
            )
        }

        "GetParameters" => {
            let mut names = Vec::new();
            if let Some(arr) = json_val.get("Names").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        names.push(s.to_string());
                    }
                }
            }
            let with_decryption = json_val
                .get("WithDecryption")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let (params, invalid) = engine.get_parameters(&names, with_decryption)?;
            let params_json: Vec<_> = params.iter().map(format_parameter_json).collect();

            make_json_response(
                json!({
                    "InvalidParameters": invalid,
                    "Parameters": params_json
                }),
                StatusCode::OK,
            )
        }

        "GetParametersByPath" => {
            let path = json_val
                .get("Path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::ssm_bad_request(
                        "ValidationException",
                        "Parameter Path is required.",
                    )
                })?;
            let recursive = json_val
                .get("Recursive")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let with_decryption = json_val
                .get("WithDecryption")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let max_results = json_val
                .get("MaxResults")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            let params =
                engine.get_parameters_by_path(path, recursive, with_decryption, max_results)?;
            let params_json: Vec<_> = params.iter().map(format_parameter_json).collect();

            make_json_response(
                json!({
                    "Parameters": params_json
                }),
                StatusCode::OK,
            )
        }

        "DeleteParameter" => {
            let name = json_val
                .get("Name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::ssm_bad_request(
                        "ValidationException",
                        "Parameter Name is required.",
                    )
                })?;
            engine.delete_parameter(name)?;
            make_json_response(json!({}), StatusCode::OK)
        }

        "DeleteParameters" => {
            let mut names = Vec::new();
            if let Some(arr) = json_val.get("Names").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        names.push(s.to_string());
                    }
                }
            }
            let (deleted, invalid) = engine.delete_parameters(&names)?;
            make_json_response(
                json!({
                    "DeletedParameters": deleted,
                    "InvalidParameters": invalid
                }),
                StatusCode::OK,
            )
        }

        "DescribeParameters" => {
            let list = engine.describe_parameters()?;
            let desc_json: Vec<_> = list
                .into_iter()
                .map(|p| {
                    json!({
                        "Name": p.name,
                        "Type": p.parameter_type.as_str(),
                        "Version": p.version,
                        "Description": p.description,
                        "LastModifiedDate": p.last_modified_date.timestamp(),
                        "Tier": p.tier,
                        "DataType": p.data_type
                    })
                })
                .collect();
            make_json_response(json!({ "Parameters": desc_json }), StatusCode::OK)
        }

        _ => Err(RustStackError::ssm_bad_request(
            "InvalidAction",
            format!("Action {} is not supported by SSM.", action),
        )),
    }
}
