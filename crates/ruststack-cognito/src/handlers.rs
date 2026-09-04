use crate::state::{CognitoError, CognitoState};
use crate::types::*;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_cognito_request(
    State(state): State<CognitoState>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let path = uri.path();
    if path.ends_with("/.well-known/jwks.json") || path == "/.well-known/jwks.json" {
        return (
            StatusCode::OK,
            [("content-type", "application/json")],
            serde_json::to_string(&state.get_jwks()).unwrap(),
        )
            .into_response();
    }

    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let operation = if let Some(pos) = target.find('.') {
        &target[pos + 1..]
    } else {
        target
    };

    match operation {
        "CreateUserPool" => {
            let req: CreateUserPoolRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.create_user_pool(req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_cognito_error(e),
            }
        }
        "DescribeUserPool" => {
            let req: DescribeUserPoolRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.describe_user_pool(req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_cognito_error(e),
            }
        }
        "ListUserPools" => {
            let req: ListUserPoolsRequest = serde_json::from_slice(&body).unwrap_or_default();
            match state.list_user_pools(req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_cognito_error(e),
            }
        }
        "DeleteUserPool" => {
            let req: DeleteUserPoolRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.delete_user_pool(req) {
                Ok(()) => json_response(StatusCode::OK, json!({})),
                Err(e) => map_cognito_error(e),
            }
        }
        "CreateUserPoolClient" => {
            let req: CreateUserPoolClientRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.create_user_pool_client(req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_cognito_error(e),
            }
        }
        "DescribeUserPoolClient" => {
            let req: DescribeUserPoolClientRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.describe_user_pool_client(req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_cognito_error(e),
            }
        }
        "ListUserPoolClients" => {
            let req: ListUserPoolClientsRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.list_user_pool_clients(req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_cognito_error(e),
            }
        }
        "DeleteUserPoolClient" => {
            let req: DeleteUserPoolClientRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.delete_user_pool_client(req) {
                Ok(()) => json_response(StatusCode::OK, json!({})),
                Err(e) => map_cognito_error(e),
            }
        }
        "SignUp" => {
            let req: SignUpRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.sign_up(req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_cognito_error(e),
            }
        }
        "AdminCreateUser" => {
            let req: AdminCreateUserRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.admin_create_user(req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_cognito_error(e),
            }
        }
        "AdminGetUser" => {
            let req: AdminGetUserRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.admin_get_user(req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_cognito_error(e),
            }
        }
        "AdminSetUserPassword" => {
            let req: AdminSetUserPasswordRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.admin_set_user_password(req) {
                Ok(()) => json_response(StatusCode::OK, json!({})),
                Err(e) => map_cognito_error(e),
            }
        }
        "AdminDeleteUser" => {
            let req: AdminDeleteUserRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.admin_delete_user(req) {
                Ok(()) => json_response(StatusCode::OK, json!({})),
                Err(e) => map_cognito_error(e),
            }
        }
        "ListUsers" => {
            let req: ListUsersRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.list_users(req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_cognito_error(e),
            }
        }
        "InitiateAuth" => {
            let req: InitiateAuthRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.initiate_auth(req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_cognito_error(e),
            }
        }
        "AdminInitiateAuth" => {
            let req: AdminInitiateAuthRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.admin_initiate_auth(req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_cognito_error(e),
            }
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("Unknown Cognito operation: {}", operation),
        ),
    }
}

fn json_response(status: StatusCode, val: serde_json::Value) -> Response {
    (
        status,
        [("content-type", "application/x-amz-json-1.1")],
        serde_json::to_string(&val).unwrap_or_default(),
    )
        .into_response()
}

fn error_response(status: StatusCode, error_type: &str, message: &str) -> Response {
    let body = json!({
        "__type": error_type,
        "message": message
    });
    (
        status,
        [("content-type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn map_cognito_error(err: CognitoError) -> Response {
    match err {
        CognitoError::ResourceNotFound(msg) => {
            error_response(StatusCode::BAD_REQUEST, "ResourceNotFoundException", &msg)
        }
        CognitoError::UsernameExists(msg) => {
            error_response(StatusCode::BAD_REQUEST, "UsernameExistsException", &msg)
        }
        CognitoError::UserNotFound(msg) => {
            error_response(StatusCode::BAD_REQUEST, "UserNotFoundException", &msg)
        }
        CognitoError::NotAuthorized(msg) => {
            error_response(StatusCode::BAD_REQUEST, "NotAuthorizedException", &msg)
        }
        CognitoError::InvalidParameter(msg) => {
            error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &msg)
        }
        CognitoError::CodeMismatch(msg) => {
            error_response(StatusCode::BAD_REQUEST, "CodeMismatchException", &msg)
        }
    }
}
