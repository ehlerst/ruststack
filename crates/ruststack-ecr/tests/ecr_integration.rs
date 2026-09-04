use axum::extract::State;
use axum::http::HeaderMap;
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_ecr::{handle_ecr_request, EcrState};
use serde_json::json;

#[tokio::test]
async fn test_ecr_repository_and_image_lifecycle() {
    let state = EcrState::new("000000000000", "us-east-1");

    // 1. CreateRepository
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-amz-target",
        "AmazonEC2ContainerRegistry_V20150921.CreateRepository".parse().unwrap(),
    );
    let body = Bytes::from(
        json!({
            "repositoryName": "backend-service"
        })
        .to_string(),
    );
    let resp = handle_ecr_request(State(state.clone()), headers, body).await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let res: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(res["repository"]["repositoryName"], "backend-service");

    // 2. GetAuthorizationToken
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-amz-target",
        "AmazonEC2ContainerRegistry_V20150921.GetAuthorizationToken".parse().unwrap(),
    );
    let resp = handle_ecr_request(State(state.clone()), headers, Bytes::from("{}")).await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let res: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(!res["authorizationData"].as_array().unwrap().is_empty());

    // 3. PutImage
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-amz-target",
        "AmazonEC2ContainerRegistry_V20150921.PutImage".parse().unwrap(),
    );
    let body = Bytes::from(
        json!({
            "repositoryName": "backend-service",
            "imageManifest": "{\"schemaVersion\":2}",
            "imageTag": "v1.0.0"
        })
        .to_string(),
    );
    let resp = handle_ecr_request(State(state.clone()), headers, body).await;
    assert_eq!(resp.status(), http::StatusCode::OK);

    // 4. BatchGetImage
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-amz-target",
        "AmazonEC2ContainerRegistry_V20150921.BatchGetImage".parse().unwrap(),
    );
    let body = Bytes::from(
        json!({
            "repositoryName": "backend-service",
            "imageIds": [
                {"imageTag": "v1.0.0"}
            ]
        })
        .to_string(),
    );
    let resp = handle_ecr_request(State(state.clone()), headers, body).await;
    assert_eq!(resp.status(), http::StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let res: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(res["images"][0]["imageId"]["imageTag"], "v1.0.0");
}
