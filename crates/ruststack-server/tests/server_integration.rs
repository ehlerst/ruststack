use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_s3::InMemoryStorage;
use ruststack_server::{create_router, AppState};
use ruststack_sqs::SqsEngine;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn test_server_unified_routing() {
    let s3_storage = Arc::new(InMemoryStorage::new());
    let sqs_engine = Arc::new(SqsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));

    let state = AppState {
        s3_storage,
        sqs_engine,
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
}
