use crate::store::SecretsManagerEngine;
use crate::types::CreateSecretRequest;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use http_body_util::BodyExt;
use ruststack_core::RustStackError;
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_secretsmanager_request(
    engine: Arc<SecretsManagerEngine>,
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
            return make_secretsmanager_error_response(
                &RustStackError::BadRequest(e.to_string()),
                &request_id,
            );
        }
    };

    let result = handle_secretsmanager_json(
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
        Err(err) => make_secretsmanager_error_response(&err, &request_id),
    }
}

pub fn make_secretsmanager_error_response(
    err: &RustStackError,
    request_id: &str,
) -> Response<Body> {
    let status = err.status_code();
    let json_err = err.to_secretsmanager_json();
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

async fn handle_secretsmanager_json(
    engine: &SecretsManagerEngine,
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
        .strip_prefix("secretsmanager.")
        .or_else(|| target.strip_prefix("secretsmanager"))
        .unwrap_or(target);

    let json_val: Value = if body.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_slice(body).map_err(|e| RustStackError::BadRequest(e.to_string()))?
    };

    match action {
        "CreateSecret" => {
            let name = json_val
                .get("Name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::secretsmanager_bad_request(
                        "InvalidParameterException",
                        "Parameter Name is required.",
                    )
                })?
                .to_string();
            let description = json_val
                .get("Description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let kms_key_id = json_val
                .get("KmsKeyId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let secret_string = json_val
                .get("SecretString")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let secret_binary = json_val
                .get("SecretBinary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let client_request_token = json_val
                .get("ClientRequestToken")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let (arn, version_id) = engine.create_secret(CreateSecretRequest {
                name: name.clone(),
                description,
                kms_key_id,
                secret_string,
                secret_binary,
                client_request_token,
            })?;

            make_json_response(
                json!({
                    "ARN": arn,
                    "Name": name,
                    "VersionId": version_id
                }),
                StatusCode::OK,
            )
        }

        "GetSecretValue" => {
            let secret_id = json_val
                .get("SecretId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::secretsmanager_bad_request(
                        "InvalidParameterException",
                        "Parameter SecretId is required.",
                    )
                })?;
            let version_id = json_val.get("VersionId").and_then(|v| v.as_str());
            let version_stage = json_val.get("VersionStage").and_then(|v| v.as_str());

            let ver = engine.get_secret_value(secret_id, version_id, version_stage)?;
            let desc = engine.describe_secret(secret_id)?;

            let mut resp = json!({
                "ARN": desc.arn,
                "Name": desc.name,
                "VersionId": ver.version_id,
                "VersionStages": ver.version_stages,
                "CreatedDate": ver.created_date.timestamp()
            });

            if let Some(ref s) = ver.secret_string {
                resp["SecretString"] = json!(s);
            }
            if let Some(ref b) = ver.secret_binary {
                resp["SecretBinary"] = json!(b);
            }

            make_json_response(resp, StatusCode::OK)
        }

        "PutSecretValue" => {
            let secret_id = json_val
                .get("SecretId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::secretsmanager_bad_request(
                        "InvalidParameterException",
                        "Parameter SecretId is required.",
                    )
                })?;
            let secret_string = json_val
                .get("SecretString")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let secret_binary = json_val
                .get("SecretBinary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let token = json_val
                .get("ClientRequestToken")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let stages: Option<Vec<String>> = json_val
                .get("VersionStages")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                });

            let (arn, new_vid, version_stages) =
                engine.put_secret_value(secret_id, secret_string, secret_binary, token, stages)?;
            let desc = engine.describe_secret(secret_id)?;

            make_json_response(
                json!({
                    "ARN": arn,
                    "Name": desc.name,
                    "VersionId": new_vid,
                    "VersionStages": version_stages
                }),
                StatusCode::OK,
            )
        }

        "UpdateSecret" => {
            let secret_id = json_val
                .get("SecretId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::secretsmanager_bad_request(
                        "InvalidParameterException",
                        "Parameter SecretId is required.",
                    )
                })?;
            let description = json_val
                .get("Description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let kms_key_id = json_val
                .get("KmsKeyId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let secret_string = json_val
                .get("SecretString")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let secret_binary = json_val
                .get("SecretBinary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let token = json_val
                .get("ClientRequestToken")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let arn = engine.update_secret(
                secret_id,
                description,
                kms_key_id,
                secret_string,
                secret_binary,
                token,
            )?;
            let desc = engine.describe_secret(secret_id)?;

            make_json_response(
                json!({
                    "ARN": arn,
                    "Name": desc.name
                }),
                StatusCode::OK,
            )
        }

        "DeleteSecret" => {
            let secret_id = json_val
                .get("SecretId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::secretsmanager_bad_request(
                        "InvalidParameterException",
                        "Parameter SecretId is required.",
                    )
                })?;
            let force = json_val
                .get("ForceDeleteWithoutRecovery")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let (arn, name, del_date) = engine.delete_secret(secret_id, force)?;
            let mut resp = json!({
                "ARN": arn,
                "Name": name
            });
            if let Some(d) = del_date {
                resp["DeletionDate"] = json!(d);
            }
            make_json_response(resp, StatusCode::OK)
        }

        "DescribeSecret" => {
            let secret_id = json_val
                .get("SecretId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RustStackError::secretsmanager_bad_request(
                        "InvalidParameterException",
                        "Parameter SecretId is required.",
                    )
                })?;
            let s = engine.describe_secret(secret_id)?;
            let mut versions_map = serde_json::Map::new();
            for (vid, ver) in &s.versions {
                versions_map.insert(vid.clone(), json!(ver.version_stages));
            }

            make_json_response(
                json!({
                    "ARN": s.arn,
                    "Name": s.name,
                    "Description": s.description,
                    "KmsKeyId": s.kms_key_id,
                    "LastChangedDate": s.last_changed_date.map(|d| d.timestamp()),
                    "LastAccessedDate": s.last_accessed_date.map(|d| d.timestamp()),
                    "VersionIdsToStages": Value::Object(versions_map)
                }),
                StatusCode::OK,
            )
        }

        "ListSecrets" => {
            let max_results = json_val
                .get("MaxResults")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            let list = engine.list_secrets(max_results)?;
            let list_json: Vec<_> = list
                .into_iter()
                .map(|s| {
                    json!({
                        "ARN": s.arn,
                        "Name": s.name,
                        "Description": s.description,
                        "KmsKeyId": s.kms_key_id
                    })
                })
                .collect();

            make_json_response(json!({ "SecretList": list_json }), StatusCode::OK)
        }

        "GetRandomPassword" => {
            let length = json_val
                .get("PasswordLength")
                .and_then(|v| v.as_u64())
                .unwrap_or(32) as usize;
            let pwd: String = (0..length)
                .map(|_| {
                    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=";
                    chars[rand_index(chars.len())] as char
                })
                .collect();
            make_json_response(json!({ "RandomPassword": pwd }), StatusCode::OK)
        }

        _ => Err(RustStackError::secretsmanager_bad_request(
            "InvalidAction",
            format!("Action {} is not supported by Secrets Manager.", action),
        )),
    }
}

fn rand_index(len: usize) -> usize {
    (uuid::Uuid::new_v4().as_u128() as usize) % len
}
