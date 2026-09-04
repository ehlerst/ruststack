use crate::state::BedrockState;
use crate::types::*;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Method, Response, StatusCode, Uri};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_bedrock_request(
    State(state): State<BedrockState>,
    method: Method,
    uri: Uri,
    _headers: HeaderMap,
    body_bytes: Bytes,
) -> Response<Body> {
    let path = uri.path();

    if method == Method::GET && (path == "/foundation-models" || path.ends_with("/foundation-models")) {
        let models = state.list_foundation_models();
        let resp = ListFoundationModelsResponse {
            model_summaries: models,
        };
        return Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&resp).unwrap()))
            .unwrap();
    }

    if method == Method::GET && (path.starts_with("/foundation-models/") || path.contains("/foundation-models/")) {
        let model_id = path.split("/foundation-models/").nth(1).unwrap_or("").trim();
        match state.get_foundation_model(model_id) {
            Ok(model) => {
                let resp = GetFoundationModelResponse {
                    model_details: model,
                };
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&resp).unwrap()))
                    .unwrap();
            }
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "message": e.to_string() }).to_string()))
                    .unwrap();
            }
        }
    }

    if method == Method::POST && path.contains("/model/") && (path.ends_with("/invoke") || path.ends_with("/invoke-with-response-stream") || path.ends_with("/converse")) {
        let after_model = path.split("/model/").nth(1).unwrap_or("");
        let model_id = if let Some(idx) = after_model.find('/') {
            &after_model[..idx]
        } else {
            after_model
        };

        let req_json: serde_json::Value = if body_bytes.is_empty() {
            json!({})
        } else {
            match serde_json::from_slice(&body_bytes) {
                Ok(v) => v,
                Err(e) => {
                    return Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .header("content-type", "application/json")
                        .body(Body::from(json!({ "message": format!("Invalid JSON: {}", e) }).to_string()))
                        .unwrap();
                }
            }
        };

        match state.invoke_model(model_id, req_json) {
            Ok(result) => {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&result).unwrap()))
                    .unwrap();
            }
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "message": e.to_string() }).to_string()))
                    .unwrap();
            }
        }
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("content-type", "application/json")
        .body(Body::from(json!({ "message": "Unknown Bedrock operation or path" }).to_string()))
        .unwrap()
}
