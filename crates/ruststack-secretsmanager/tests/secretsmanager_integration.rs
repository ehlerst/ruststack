use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_secretsmanager::{handle_secretsmanager_request, SecretsManagerEngine};
use serde_json::Value;
use std::sync::Arc;

fn setup_secretsmanager() -> Arc<SecretsManagerEngine> {
    Arc::new(SecretsManagerEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ))
}

#[tokio::test]
async fn test_secretsmanager_lifecycle() {
    let sm = setup_secretsmanager();

    // 1. Create Secret
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "secretsmanager.CreateSecret")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Name": "my-database-secret",
                "Description": "PostgreSQL credentials",
                "SecretString": "{\"user\":\"dbadmin\",\"pass\":\"initial123\"}"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_secretsmanager_request(sm.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert!(val["ARN"].as_str().unwrap().contains("my-database-secret"));
    let initial_vid = val["VersionId"].as_str().unwrap().to_string();

    // 2. Get Secret Value
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "secretsmanager.GetSecretValue")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "SecretId": "my-database-secret"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_secretsmanager_request(sm.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        val["SecretString"].as_str().unwrap(),
        "{\"user\":\"dbadmin\",\"pass\":\"initial123\"}"
    );

    // 3. Put Secret Value (rotate/update with new version)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "secretsmanager.PutSecretValue")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "SecretId": "my-database-secret",
                "SecretString": "{\"user\":\"dbadmin\",\"pass\":\"rotated456\"}"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_secretsmanager_request(sm.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let new_vid = val["VersionId"].as_str().unwrap().to_string();
    assert_ne!(new_vid, initial_vid);

    // 4. Get Secret Value (current should now return new value)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "secretsmanager.GetSecretValue")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "SecretId": "my-database-secret"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_secretsmanager_request(sm.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        val["SecretString"].as_str().unwrap(),
        "{\"user\":\"dbadmin\",\"pass\":\"rotated456\"}"
    );

    // 5. Get Secret Value for AWSPREVIOUS (should return initial value)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "secretsmanager.GetSecretValue")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "SecretId": "my-database-secret",
                "VersionStage": "AWSPREVIOUS"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_secretsmanager_request(sm.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        val["SecretString"].as_str().unwrap(),
        "{\"user\":\"dbadmin\",\"pass\":\"initial123\"}"
    );

    // 6. Delete Secret
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "secretsmanager.DeleteSecret")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "SecretId": "my-database-secret",
                "ForceDeleteWithoutRecovery": true
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_secretsmanager_request(sm.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
