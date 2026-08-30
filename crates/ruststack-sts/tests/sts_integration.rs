use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_sts::{handle_sts_request, StsEngine};
use serde_json::Value;
use std::sync::Arc;

fn setup_sts() -> Arc<StsEngine> {
    Arc::new(StsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ))
}

#[tokio::test]
async fn test_sts_get_caller_identity_query() {
    let sts = setup_sts();

    let req = Request::builder()
        .method(Method::POST)
        .uri("/?Action=GetCallerIdentity&Version=2011-06-15")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::empty())
        .unwrap();

    let resp = handle_sts_request(sts, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("<Account>000000000000</Account>"));
    assert!(body_str.contains("<Arn>arn:aws:iam::000000000000:root</Arn>"));
}

#[tokio::test]
async fn test_sts_get_caller_identity_json() {
    let sts = setup_sts();

    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header(
            "x-amz-target",
            "AWSSecurityTokenServiceV20110615.GetCallerIdentity",
        )
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(r#"{}"#))
        .unwrap();

    let resp = handle_sts_request(sts, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["Account"].as_str().unwrap(), "000000000000");
    assert_eq!(
        val["Arn"].as_str().unwrap(),
        "arn:aws:iam::000000000000:root"
    );
}

#[tokio::test]
async fn test_sts_assume_role_query() {
    let sts = setup_sts();

    let form_body = "Action=AssumeRole&RoleArn=arn:aws:iam::000000000000:role/deploy-role&RoleSessionName=ci-session&Version=2011-06-15";
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(form_body))
        .unwrap();

    let resp = handle_sts_request(sts, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("<AccessKeyId>ASIA"));
    assert!(body_str.contains("<AssumedRoleId>ASIA"));
    assert!(body_str.contains("ci-session"));
}
