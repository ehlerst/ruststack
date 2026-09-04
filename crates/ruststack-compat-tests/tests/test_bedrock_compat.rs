use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_bedrock_foundation_models_and_inference_compat() {
    let client = RustStackTestClient::new();

    // 1. List Foundation Models
    let (status, body) = client.call_rest(http::Method::GET, "/foundation-models", None).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let models = json["modelSummaries"].as_array().unwrap();
    assert!(models.iter().any(|m| m["modelId"] == "anthropic.claude-3-5-sonnet-20240620-v1:0"));
    assert!(models.iter().any(|m| m["modelId"] == "meta.llama3-70b-instruct-v1:0"));

    // 2. Invoke Claude 3.5 Sonnet
    let req = json!({
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 50,
        "messages": [
            { "role": "user", "content": "Tell me a joke about Rust compilers." }
        ]
    });

    let (status, body) = client
        .call_rest(
            http::Method::POST,
            "/model/anthropic.claude-3-5-sonnet-20240620-v1:0/invoke",
            Some(req.to_string()),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["role"], "assistant");
    assert!(json["content"][0]["text"].as_str().unwrap().contains("RustStack Bedrock Response"));
}
