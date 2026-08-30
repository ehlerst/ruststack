use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_ssm::{handle_ssm_request, SsmEngine};
use serde_json::Value;
use std::sync::Arc;

fn setup_ssm() -> Arc<SsmEngine> {
    Arc::new(SsmEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ))
}

#[tokio::test]
async fn test_ssm_parameter_crud() {
    let ssm = setup_ssm();

    // 1. Put Parameter
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSSM.PutParameter")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Name": "/config/db/password",
                "Value": "supersecret123",
                "Type": "SecureString",
                "Description": "Production DB password"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_ssm_request(ssm.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["Version"].as_i64().unwrap(), 1);

    // 2. Overwrite Parameter (increment version)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSSM.PutParameter")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Name": "/config/db/password",
                "Value": "new_supersecret456",
                "Type": "SecureString",
                "Overwrite": true
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_ssm_request(ssm.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["Version"].as_i64().unwrap(), 2);

    // 3. Get Parameter
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSSM.GetParameter")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Name": "/config/db/password"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_ssm_request(ssm.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        val["Parameter"]["Value"].as_str().unwrap(),
        "new_supersecret456"
    );
    assert_eq!(val["Parameter"]["Version"].as_i64().unwrap(), 2);

    // 4. Get Parameter by Version (/config/db/password:1)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSSM.GetParameter")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Name": "/config/db/password:1"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_ssm_request(ssm.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        val["Parameter"]["Value"].as_str().unwrap(),
        "supersecret123"
    );
    assert_eq!(val["Parameter"]["Version"].as_i64().unwrap(), 1);

    // 5. Delete Parameter
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSSM.DeleteParameter")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Name": "/config/db/password"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_ssm_request(ssm.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_ssm_get_parameters_by_path() {
    let ssm = setup_ssm();

    // Seed hierarchical keys
    let keys = vec![
        ("/app/prod/api/host", "api.prod.example.com"),
        ("/app/prod/api/port", "8080"),
        ("/app/prod/db/host", "db.prod.example.com"),
        ("/app/dev/api/host", "api.dev.example.com"),
    ];

    for (k, v) in keys {
        let req = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("x-amz-target", "AmazonSSM.PutParameter")
            .header("content-type", "application/x-amz-json-1.1")
            .body(Body::from(
                serde_json::json!({
                    "Name": k,
                    "Value": v,
                    "Type": "String"
                })
                .to_string(),
            ))
            .unwrap();
        let _ = handle_ssm_request(ssm.clone(), req).await;
    }

    // GetParametersByPath recursive for /app/prod -> should find 3
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSSM.GetParametersByPath")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Path": "/app/prod",
                "Recursive": true
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_ssm_request(ssm.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let params = val["Parameters"].as_array().unwrap();
    assert_eq!(params.len(), 3);
}
