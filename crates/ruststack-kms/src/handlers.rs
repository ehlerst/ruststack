use crate::state::{KmsError, KmsState};
use crate::types::*;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_kms_request(
    State(state): State<KmsState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
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
        "CreateKey" => {
            let req: CreateKeyRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "ValidationException", &e.to_string()),
            };
            match state.create_key(req) {
                Ok(meta) => json_response(StatusCode::OK, json!({ "KeyMetadata": meta })),
                Err(e) => map_kms_error(e),
            }
        }
        "DescribeKey" => {
            let req: DescribeKeyRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "ValidationException", &e.to_string()),
            };
            match state.describe_key(req) {
                Ok(meta) => json_response(StatusCode::OK, json!({ "KeyMetadata": meta })),
                Err(e) => map_kms_error(e),
            }
        }
        "ListKeys" => {
            let req: ListKeysRequest = serde_json::from_slice(&body).unwrap_or(ListKeysRequest {
                limit: None,
                marker: None,
            });
            match state.list_keys(req) {
                Ok((keys, truncated)) => json_response(StatusCode::OK, json!({
                    "Keys": keys,
                    "Truncated": truncated
                })),
                Err(e) => map_kms_error(e),
            }
        }
        "CreateAlias" => {
            let req: CreateAliasRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "ValidationException", &e.to_string()),
            };
            match state.create_alias(req.alias_name, req.target_key_id) {
                Ok(()) => (StatusCode::OK, [("content-type", "application/x-amz-json-1.1")], "").into_response(),
                Err(e) => map_kms_error(e),
            }
        }
        "DeleteAlias" => {
            let req: DeleteAliasRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "ValidationException", &e.to_string()),
            };
            match state.delete_alias(&req.alias_name) {
                Ok(()) => (StatusCode::OK, [("content-type", "application/x-amz-json-1.1")], "").into_response(),
                Err(e) => map_kms_error(e),
            }
        }
        "ListAliases" => {
            let req: ListAliasesRequest = serde_json::from_slice(&body).unwrap_or(ListAliasesRequest {
                key_id: None,
                limit: None,
                marker: None,
            });
            match state.list_aliases(req) {
                Ok(aliases) => json_response(StatusCode::OK, json!({
                    "Aliases": aliases,
                    "Truncated": false
                })),
                Err(e) => map_kms_error(e),
            }
        }
        "Encrypt" => {
            let req: EncryptRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "ValidationException", &e.to_string()),
            };
            match state.encrypt(req) {
                Ok((cipher, key_arn)) => json_response(StatusCode::OK, json!({
                    "CiphertextBlob": cipher,
                    "KeyId": key_arn,
                    "EncryptionAlgorithm": "SYMMETRIC_DEFAULT"
                })),
                Err(e) => map_kms_error(e),
            }
        }
        "Decrypt" => {
            let req: DecryptRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "ValidationException", &e.to_string()),
            };
            match state.decrypt(req) {
                Ok((plain, key_arn)) => json_response(StatusCode::OK, json!({
                    "Plaintext": plain,
                    "KeyId": key_arn,
                    "EncryptionAlgorithm": "SYMMETRIC_DEFAULT"
                })),
                Err(e) => map_kms_error(e),
            }
        }
        "GenerateDataKey" => {
            let req: GenerateDataKeyRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "ValidationException", &e.to_string()),
            };
            match state.generate_data_key(req) {
                Ok((plain, cipher, key_arn)) => json_response(StatusCode::OK, json!({
                    "Plaintext": plain,
                    "CiphertextBlob": cipher,
                    "KeyId": key_arn
                })),
                Err(e) => map_kms_error(e),
            }
        }
        "DisableKey" => {
            let req: KeyIdOnlyRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "ValidationException", &e.to_string()),
            };
            match state.disable_key(&req.key_id) {
                Ok(()) => (StatusCode::OK, [("content-type", "application/x-amz-json-1.1")], "").into_response(),
                Err(e) => map_kms_error(e),
            }
        }
        "EnableKey" => {
            let req: KeyIdOnlyRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "ValidationException", &e.to_string()),
            };
            match state.enable_key(&req.key_id) {
                Ok(()) => (StatusCode::OK, [("content-type", "application/x-amz-json-1.1")], "").into_response(),
                Err(e) => map_kms_error(e),
            }
        }
        "ScheduleKeyDeletion" => {
            let req: ScheduleKeyDeletionRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "ValidationException", &e.to_string()),
            };
            match state.schedule_key_deletion(req) {
                Ok((key_arn, deletion_date)) => json_response(StatusCode::OK, json!({
                    "KeyId": key_arn,
                    "DeletionDate": deletion_date,
                    "KeyState": "PendingDeletion"
                })),
                Err(e) => map_kms_error(e),
            }
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("Unknown KMS operation: {}", operation),
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

fn map_kms_error(err: KmsError) -> Response {
    match err {
        KmsError::NotFound(msg) => error_response(StatusCode::NOT_FOUND, "NotFoundException", &msg),
        KmsError::AlreadyExists(msg) => error_response(StatusCode::CONFLICT, "AlreadyExistsException", &msg),
        KmsError::Disabled(msg) => error_response(StatusCode::BAD_REQUEST, "DisabledException", &msg),
        KmsError::InvalidCiphertext(msg) => error_response(StatusCode::BAD_REQUEST, "InvalidCiphertextException", &msg),
        KmsError::InvalidArn(msg) => error_response(StatusCode::BAD_REQUEST, "InvalidArnException", &msg),
        KmsError::Validation(msg) => error_response(StatusCode::BAD_REQUEST, "ValidationException", &msg),
    }
}
