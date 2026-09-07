use crate::state::PipesState;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_pipes_request(
    State(state): State<PipesState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .map(|t| t.split('.').next_back().unwrap_or(t).to_string());

    let val: serde_json::Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let path = uri.path();
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();

    // Check action from target or REST path
    let action = if let Some(t) = target {
        t
    } else if parts.first() == Some(&"v1") && parts.get(1) == Some(&"pipes") {
        if parts.len() == 2 {
            match method {
                Method::GET => "ListPipes".to_string(),
                _ => "Unknown".to_string(),
            }
        } else if parts.len() == 3 {
            match method {
                Method::POST | Method::PUT => "CreatePipe".to_string(),
                Method::GET => "DescribePipe".to_string(),
                Method::DELETE => "DeletePipe".to_string(),
                _ => "Unknown".to_string(),
            }
        } else if parts.len() == 4 && parts.get(3) == Some(&"start") {
            "StartPipe".to_string()
        } else if parts.len() == 4 && parts.get(3) == Some(&"stop") {
            "StopPipe".to_string()
        } else {
            "Unknown".to_string()
        }
    } else {
        "Unknown".to_string()
    };

    let name_from_path = if parts.first() == Some(&"v1") && parts.get(1) == Some(&"pipes") {
        parts.get(2).map(|s| s.to_string())
    } else {
        None
    };

    match action.as_str() {
        "CreatePipe" => {
            let name = name_from_path
                .or_else(|| val["Name"].as_str().map(String::from))
                .unwrap_or_default();
            let source = val["Source"].as_str().unwrap_or_default().to_string();
            let target = val["Target"].as_str().unwrap_or_default().to_string();
            let role_arn = val["RoleArn"].as_str().unwrap_or_default().to_string();
            let desired_state = val["DesiredState"].as_str().map(String::from);
            let enrichment = val["Enrichment"].as_str().map(String::from);
            let description = val["Description"].as_str().map(String::from);
            let source_parameters = val.get("SourceParameters").cloned();
            let target_parameters = val.get("TargetParameters").cloned();
            let tags = val.get("Tags").and_then(|t| {
                serde_json::from_value::<std::collections::HashMap<String, String>>(t.clone()).ok()
            });

            match state.create_pipe(
                name,
                source,
                target,
                role_arn,
                desired_state,
                enrichment,
                description,
                source_parameters,
                target_parameters,
                tags,
            ) {
                Ok(pipe) => json_response(StatusCode::OK, json!(pipe)),
                Err(e) => error_response(
                    StatusCode::BAD_REQUEST,
                    "PipeAlreadyExistsException",
                    &e.to_string(),
                ),
            }
        }
        "DescribePipe" => {
            let name = name_from_path
                .or_else(|| val["Name"].as_str().map(String::from))
                .unwrap_or_default();
            match state.describe_pipe(&name) {
                Ok(pipe) => json_response(StatusCode::OK, json!(pipe)),
                Err(e) => {
                    error_response(StatusCode::NOT_FOUND, "NotFoundException", &e.to_string())
                }
            }
        }
        "ListPipes" => {
            let list = state.list_pipes();
            json_response(StatusCode::OK, json!({ "Pipes": list }))
        }
        "DeletePipe" => {
            let name = name_from_path
                .or_else(|| val["Name"].as_str().map(String::from))
                .unwrap_or_default();
            match state.delete_pipe(&name) {
                Ok(pipe) => json_response(StatusCode::OK, json!(pipe)),
                Err(e) => {
                    error_response(StatusCode::NOT_FOUND, "NotFoundException", &e.to_string())
                }
            }
        }
        "StartPipe" => {
            let name = name_from_path
                .or_else(|| val["Name"].as_str().map(String::from))
                .unwrap_or_default();
            match state.start_pipe(&name) {
                Ok(pipe) => json_response(StatusCode::OK, json!(pipe)),
                Err(e) => {
                    error_response(StatusCode::NOT_FOUND, "NotFoundException", &e.to_string())
                }
            }
        }
        "StopPipe" => {
            let name = name_from_path
                .or_else(|| val["Name"].as_str().map(String::from))
                .unwrap_or_default();
            match state.stop_pipe(&name) {
                Ok(pipe) => json_response(StatusCode::OK, json!(pipe)),
                Err(e) => {
                    error_response(StatusCode::NOT_FOUND, "NotFoundException", &e.to_string())
                }
            }
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("Action '{}' is not supported by EventBridge Pipes", action),
        ),
    }
}

fn json_response(status: StatusCode, body: serde_json::Value) -> Response {
    (
        status,
        [("content-type", "application/json")],
        body.to_string(),
    )
        .into_response()
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        [("content-type", "application/json")],
        json!({
            "__type": code,
            "message": message
        })
        .to_string(),
    )
        .into_response()
}
