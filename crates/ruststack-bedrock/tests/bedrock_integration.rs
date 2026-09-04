use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_bedrock::{handle_bedrock_request, BedrockState};

#[tokio::test]
async fn test_bedrock_models_and_invoke() {
    let state = BedrockState::new("000000000000".to_string(), "us-east-1".to_string());
    let headers = HeaderMap::new();

    // 1. List Foundation Models
    let uri: Uri = "/foundation-models".parse().unwrap();
    let resp = handle_bedrock_request(State(state.clone()), Method::GET, uri, headers.clone(), Bytes::new()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["modelSummaries"].as_array().unwrap().len() >= 5);

    // 2. Invoke Claude Model
    let uri: Uri = "/model/anthropic.claude-3-5-sonnet-20240620-v1:0/invoke".parse().unwrap();
    let req_body = Bytes::from(serde_json::json!({
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 100,
        "messages": [
            { "role": "user", "content": "Hello RustStack Bedrock" }
        ]
    }).to_string());

    let resp = handle_bedrock_request(State(state.clone()), Method::POST, uri, headers.clone(), req_body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["role"], "assistant");
    assert!(json["content"][0]["text"].as_str().unwrap().contains("Hello RustStack Bedrock"));
}
