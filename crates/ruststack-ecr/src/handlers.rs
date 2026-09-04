use crate::state::{EcrError, EcrState};
use crate::types::*;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_ecr_request(
    State(state): State<EcrState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let action = if let Some(pos) = target.rfind('.') {
        &target[pos + 1..]
    } else {
        target
    };

    match action {
        "CreateRepository" => {
            let req: CreateRepositoryRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.create_repository(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        "DescribeRepositories" => {
            let req: DescribeRepositoriesRequest = serde_json::from_slice(&body).unwrap_or_default();
            match state.describe_repositories(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        "DeleteRepository" => {
            let req: DeleteRepositoryRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.delete_repository(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        "GetAuthorizationToken" => {
            let resp = state.get_authorization_token();
            ok_json(&resp)
        }
        "PutImage" => {
            let req: PutImageRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.put_image(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        "BatchGetImage" => {
            let req: BatchGetImageRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.batch_get_image(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        "ListImages" => {
            let req: ListImagesRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.list_images(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        "BatchDeleteImage" => {
            let req: BatchDeleteImageRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.batch_delete_image(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        _ => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("content-type", "application/x-amz-json-1.1")
            .body(Body::from(
                json!({
                    "__type": "InvalidAction",
                    "message": format!("Unknown action {}", action)
                })
                .to_string(),
            ))
            .unwrap(),
    }
}

fn ok_json<T: serde::Serialize>(val: &T) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(serde_json::to_string(val).unwrap()))
        .unwrap()
}

fn bad_request(msg: &str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            json!({
                "__type": "InvalidParameterException",
                "message": msg
            })
            .to_string(),
        ))
        .unwrap()
}

fn error_response(err: EcrError) -> Response<Body> {
    let (status, type_name, msg) = match err {
        EcrError::RepositoryAlreadyExists(m) => (StatusCode::BAD_REQUEST, "RepositoryAlreadyExistsException", format!("The repository with name '{}' already exists", m)),
        EcrError::RepositoryNotFound(m) => (StatusCode::BAD_REQUEST, "RepositoryNotFoundException", format!("The repository with name '{}' does not exist", m)),
        EcrError::ImageNotFound(m) => (StatusCode::BAD_REQUEST, "ImageNotFoundException", m),
        EcrError::InvalidParameter(m) => (StatusCode::BAD_REQUEST, "InvalidParameterException", m),
    };

    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            json!({
                "__type": type_name,
                "message": msg
            })
            .to_string(),
        ))
        .unwrap()
}
