use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_eventbridge::EventBridgeEngine;
use ruststack_s3::InMemoryStorage;
use ruststack_server::{create_router, AppState};
use ruststack_sns::SnsEngine;
use ruststack_sqs::SqsEngine;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn test_server_unified_routing() {
    let s3_storage = Arc::new(InMemoryStorage::new());
    let sqs_engine = Arc::new(SqsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let sns_engine = Arc::new(SnsEngine::new(
        sqs_engine.clone(),
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let eventbridge_engine = Arc::new(EventBridgeEngine::new(
        sqs_engine.clone(),
        sns_engine.clone(),
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));

    let state = AppState {
        s3_storage,
        sqs_engine,
        sns_engine,
        eventbridge_engine,
        region: "us-east-1".to_string(),
        account_id: "000000000000".to_string(),
    };

    let app = create_router(state);

    // 1. Health check
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/_ruststack/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(body.as_ref().starts_with(b"{\"status\": \"running\""));

    // 2. S3 create bucket via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/unified-bucket")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. SQS create queue via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AmazonSQS.CreateQueue")
                .header("content-type", "application/x-amz-json-1.0")
                .body(Body::from(r#"{"QueueName": "unified-queue"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. SNS create topic via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AmazonSNS.CreateTopic")
                .header("content-type", "application/x-amz-json-1.0")
                .body(Body::from(r#"{"Name": "unified-topic"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. EventBridge PutEvents via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AWSEvents.PutEvents")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(
                    serde_json::json!({
                        "Entries": [
                            {
                                "Source": "unified.test",
                                "DetailType": "HealthEvent",
                                "Detail": "{\"status\": \"ok\"}"
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["FailedEntryCount"].as_u64().unwrap(), 0);
}
