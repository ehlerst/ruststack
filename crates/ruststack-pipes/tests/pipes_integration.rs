use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_pipes::{handle_pipes_request, PipesState};
use serde_json::json;

#[tokio::test]
async fn test_pipes_lifecycle() {
    let state = PipesState::new("000000000000".to_string(), "us-east-1".to_string());
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());

    // 1. Create Pipe
    let body = Bytes::from(
        json!({
            "Name": "sqs-to-lambda",
            "Source": "arn:aws:sqs:us-east-1:000000000000:my-queue",
            "Target": "arn:aws:lambda:us-east-1:000000000000:function:process-item",
            "RoleArn": "arn:aws:iam::000000000000:role/PipeRole",
            "DesiredState": "RUNNING"
        })
        .to_string(),
    );
    let uri: Uri = "/v1/pipes/sqs-to-lambda".parse().unwrap();
    let resp = handle_pipes_request(
        State(state.clone()),
        Method::POST,
        uri,
        headers.clone(),
        body,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["Name"], "sqs-to-lambda");
    assert_eq!(val["CurrentState"], "RUNNING");

    // 2. Describe Pipe
    let uri: Uri = "/v1/pipes/sqs-to-lambda".parse().unwrap();
    let resp = handle_pipes_request(
        State(state.clone()),
        Method::GET,
        uri,
        headers.clone(),
        Bytes::new(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["Source"], "arn:aws:sqs:us-east-1:000000000000:my-queue");

    // 3. Stop Pipe
    let uri: Uri = "/v1/pipes/sqs-to-lambda/stop".parse().unwrap();
    let resp = handle_pipes_request(
        State(state.clone()),
        Method::POST,
        uri,
        headers.clone(),
        Bytes::new(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["CurrentState"], "STOPPED");

    // 4. List Pipes
    let uri: Uri = "/v1/pipes".parse().unwrap();
    let resp = handle_pipes_request(
        State(state.clone()),
        Method::GET,
        uri,
        headers.clone(),
        Bytes::new(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["Pipes"].as_array().unwrap().len(), 1);

    // 5. Delete Pipe
    let uri: Uri = "/v1/pipes/sqs-to-lambda".parse().unwrap();
    let resp = handle_pipes_request(
        State(state.clone()),
        Method::DELETE,
        uri,
        headers.clone(),
        Bytes::new(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}
