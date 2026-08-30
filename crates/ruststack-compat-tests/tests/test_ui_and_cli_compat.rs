use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_server::{create_router, AppState};
use std::sync::Arc;
use tower::ServiceExt;

fn setup_test_app() -> axum::Router {
    let region = "us-east-1".to_string();
    let account_id = "000000000000".to_string();

    let s3_storage = Arc::new(ruststack_s3::InMemoryStorage::new());
    let sqs_engine = Arc::new(ruststack_sqs::SqsEngine::new(
        account_id.clone(),
        region.clone(),
    ));
    let sns_engine = Arc::new(ruststack_sns::SnsEngine::new(
        sqs_engine.clone(),
        account_id.clone(),
        region.clone(),
    ));
    let eventbridge_engine = Arc::new(ruststack_eventbridge::EventBridgeEngine::new(
        sqs_engine.clone(),
        sns_engine.clone(),
        account_id.clone(),
        region.clone(),
    ));
    let ssm_engine = Arc::new(ruststack_ssm::SsmEngine::new(
        account_id.clone(),
        region.clone(),
    ));
    let secretsmanager_engine = Arc::new(ruststack_secretsmanager::SecretsManagerEngine::new(
        account_id.clone(),
        region.clone(),
    ));
    let sts_engine = Arc::new(ruststack_sts::StsEngine::new(
        account_id.clone(),
        region.clone(),
    ));
    let dynamodb_engine = Arc::new(ruststack_dynamodb::DynamoDbEngine::new(
        account_id.clone(),
        region.clone(),
    ));
    let chaos_engine = Arc::new(ruststack_core::ChaosEngine::new());

    let app_state = AppState {
        s3_storage,
        sqs_engine,
        sns_engine,
        eventbridge_engine,
        ssm_engine,
        secretsmanager_engine,
        sts_engine,
        dynamodb_engine,
        chaos_engine,
        region,
        account_id,
    };

    create_router(app_state)
}

#[tokio::test]
async fn test_embedded_ui_delivery() {
    let app = setup_test_app();

    // 1. Direct /_ruststack/ui endpoint
    let req = Request::builder()
        .method(Method::GET)
        .uri("/_ruststack/ui")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&body_bytes);
    assert!(html.contains("RustStack Cloud Console"));
    assert!(html.contains("Chaos Studio"));
    assert!(html.contains("Snapshots"));

    // 2. Direct /_ruststack/ui/ endpoint
    let req = Request::builder()
        .method(Method::GET)
        .uri("/_ruststack/ui/")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Browser root request with Accept: text/html
    let req = Request::builder()
        .method(Method::GET)
        .uri("/")
        .header("accept", "text/html,application/xhtml+xml")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&body_bytes);
    assert!(html.contains("RustStack Cloud Console"));
}
