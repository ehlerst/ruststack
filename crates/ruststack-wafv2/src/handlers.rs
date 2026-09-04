use crate::state::Wafv2State;
use crate::types::VisibilityConfig;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode};
use bytes::Bytes;
use serde_json::{json, Value};

pub async fn handle_wafv2_request(
    State(state): State<Wafv2State>,
    headers: HeaderMap,
    body_bytes: Bytes,
) -> Response<Body> {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let action = target
        .split('.')
        .nth(1)
        .or_else(|| target.split('#').nth(1))
        .unwrap_or(target);

    let val: Value = serde_json::from_slice(&body_bytes).unwrap_or(Value::Null);

    match action {
        "CreateWebACL" => {
            let name = val["Name"].as_str().unwrap_or_default().to_string();
            let scope = val["Scope"].as_str().unwrap_or("REGIONAL");
            let default_action = val["DefaultAction"].clone();
            let description = val["Description"].as_str().map(String::from);
            let rules = val["Rules"].as_array().cloned();
            let vis_cfg: VisibilityConfig = match serde_json::from_value(val["VisibilityConfig"].clone()) {
                Ok(c) => c,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "WAFInvalidParameterException", &e.to_string()),
            };

            match state.create_web_acl(name, scope, default_action, description, rules, vis_cfg) {
                Ok(summary) => json_response(StatusCode::OK, json!({ "Summary": summary })),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "WAFDuplicateItemException", &e.to_string()),
            }
        }
        "GetWebACL" => {
            let name = val["Name"].as_str().unwrap_or_default();
            let id = val["Id"].as_str().unwrap_or_default();
            match state.get_web_acl(name, id) {
                Ok(acl) => {
                    let lock_token = acl.lock_token.clone();
                    json_response(StatusCode::OK, json!({
                        "WebACL": acl,
                        "LockToken": lock_token
                    }))
                }
                Err(e) => error_response(StatusCode::NOT_FOUND, "WAFNonexistentItemException", &e.to_string()),
            }
        }
        "ListWebACLs" => {
            let acls = state.list_web_acls();
            json_response(StatusCode::OK, json!({ "WebACLs": acls }))
        }
        "DeleteWebACL" => {
            let id = val["Id"].as_str().unwrap_or_default();
            match state.delete_web_acl(id) {
                Ok(_) => json_response(StatusCode::OK, json!({})),
                Err(e) => error_response(StatusCode::NOT_FOUND, "WAFNonexistentItemException", &e.to_string()),
            }
        }
        "AssociateWebACL" => {
            let web_acl_arn = val["WebACLArn"].as_str().unwrap_or_default().to_string();
            let resource_arn = val["ResourceArn"].as_str().unwrap_or_default().to_string();
            state.associate_web_acl(web_acl_arn, resource_arn);
            json_response(StatusCode::OK, json!({}))
        }
        "DisassociateWebACL" => {
            let resource_arn = val["ResourceArn"].as_str().unwrap_or_default();
            state.disassociate_web_acl(resource_arn);
            json_response(StatusCode::OK, json!({}))
        }
        "GetWebACLForResource" => {
            let resource_arn = val["ResourceArn"].as_str().unwrap_or_default();
            if let Some(acl) = state.get_web_acl_for_resource(resource_arn) {
                json_response(StatusCode::OK, json!({ "WebACL": acl }))
            } else {
                json_response(StatusCode::OK, json!({}))
            }
        }
        "CreateIPSet" => {
            let name = val["Name"].as_str().unwrap_or_default().to_string();
            let scope = val["Scope"].as_str().unwrap_or("REGIONAL");
            let description = val["Description"].as_str().map(String::from);
            let ip_version = val["IPAddressVersion"].as_str().unwrap_or("IPV4").to_string();
            let addresses = val["Addresses"]
                .as_array()
                .map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect())
                .unwrap_or_default();

            match state.create_ip_set(name, scope, description, ip_version, addresses) {
                Ok(summary) => json_response(StatusCode::OK, json!({ "Summary": summary })),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "WAFDuplicateItemException", &e.to_string()),
            }
        }
        "GetIPSet" => {
            let name = val["Name"].as_str().unwrap_or_default();
            let id = val["Id"].as_str().unwrap_or_default();
            match state.get_ip_set(name, id) {
                Ok(set) => {
                    let lock_token = set.lock_token.clone();
                    json_response(StatusCode::OK, json!({
                        "IPSet": set,
                        "LockToken": lock_token
                    }))
                }
                Err(e) => error_response(StatusCode::NOT_FOUND, "WAFNonexistentItemException", &e.to_string()),
            }
        }
        "ListIPSets" => {
            let sets = state.list_ip_sets();
            json_response(StatusCode::OK, json!({ "IPSets": sets }))
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("Unknown action {}", action),
        ),
    }
}

fn json_response(status: StatusCode, val: Value) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(val.to_string()))
        .unwrap()
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response<Body> {
    let err = json!({
        "__type": code,
        "message": message
    });
    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(err.to_string()))
        .unwrap()
}
