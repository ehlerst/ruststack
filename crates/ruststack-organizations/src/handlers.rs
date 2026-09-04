use crate::state::OrganizationsState;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode};
use bytes::Bytes;
use serde_json::{json, Value};

pub async fn handle_organizations_request(
    State(state): State<OrganizationsState>,
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
        "CreateOrganization" => {
            let feature_set = val["FeatureSet"].as_str().map(String::from);
            match state.create_organization(feature_set) {
                Ok(org) => json_response(StatusCode::OK, json!({ "Organization": org })),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "AlreadyInOrganizationException", &e.to_string()),
            }
        }
        "DescribeOrganization" => {
            match state.describe_organization() {
                Ok(org) => json_response(StatusCode::OK, json!({ "Organization": org })),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "AWSOrganizationsNotInUseException", &e.to_string()),
            }
        }
        "CreateAccount" => {
            let account_name = val["AccountName"].as_str().unwrap_or_default().to_string();
            let email = val["Email"].as_str().unwrap_or_default().to_string();

            match state.create_account(account_name, email) {
                Ok(account) => {
                    let status = json!({
                        "CreateAccountStatus": {
                            "Id": format!("car-{}", account.id),
                            "AccountName": account.name,
                            "State": "SUCCEEDED",
                            "AccountId": account.id
                        }
                    });
                    json_response(StatusCode::OK, status)
                }
                Err(e) => error_response(StatusCode::BAD_REQUEST, "DuplicateAccountException", &e.to_string()),
            }
        }
        "ListAccounts" => {
            let accounts = state.list_accounts();
            json_response(StatusCode::OK, json!({ "Accounts": accounts }))
        }
        "ListRoots" => {
            let roots = state.list_roots();
            json_response(StatusCode::OK, json!({ "Roots": roots }))
        }
        "CreateOrganizationalUnit" => {
            let parent_id = val["ParentId"].as_str().unwrap_or_default().to_string();
            let name = val["Name"].as_str().unwrap_or_default().to_string();

            match state.create_organizational_unit(parent_id, name) {
                Ok(ou) => json_response(StatusCode::OK, json!({ "OrganizationalUnit": ou })),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "InvalidInputException", &e.to_string()),
            }
        }
        "ListOrganizationalUnitsForParent" => {
            let parent_id = val["ParentId"].as_str().unwrap_or_default();
            let ous = state.list_organizational_units_for_parent(parent_id);
            json_response(StatusCode::OK, json!({ "OrganizationalUnits": ous }))
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
