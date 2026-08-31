use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::{json, Value};

#[tokio::test]
async fn test_lambda_crud_and_invocation_compat() {
    let client = RustStackTestClient::new();

    // 1. Create Function (POST /2015-03-31/functions)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/2015-03-31/functions")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "FunctionName": "process-payment",
                "Runtime": "python3.11",
                "Role": "arn:aws:iam::000000000000:role/lambda-role",
                "Handler": "index.handler",
                "Description": "Payment processor function"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = client.send_request(req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 2. Invoke Function (POST /2015-03-31/functions/process-payment/invocations)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/2015-03-31/functions/process-payment/invocations")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "amount": 250, "currency": "USD" }).to_string(),
        ))
        .unwrap();
    let resp = client.send_request(req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert!(val.is_object() || val.is_string() || val.is_null());
}
