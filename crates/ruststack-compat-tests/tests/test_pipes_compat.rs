use http::{Method, StatusCode};
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_pipes_lifecycle_compat() {
    let client = RustStackTestClient::new();

    // 1. Create Pipe
    let (status, body) = client
        .call_rest_json(
            Method::POST,
            "/v1/pipes/order-stream-pipe",
            Some(json!({
                "Source": "arn:aws:sqs:us-east-1:000000000000:orders-queue",
                "Target": "arn:aws:sns:us-east-1:000000000000:orders-topic",
                "RoleArn": "arn:aws:iam::000000000000:role/PipesRole",
                "DesiredState": "RUNNING",
                "Description": "Orders event pipeline"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["Name"], "order-stream-pipe");
    assert_eq!(body["CurrentState"], "RUNNING");
    assert!(body["Arn"].as_str().unwrap().contains("order-stream-pipe"));

    // 2. Describe Pipe
    let (status, body) = client
        .call_rest_json(Method::GET, "/v1/pipes/order-stream-pipe", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["Name"], "order-stream-pipe");
    assert_eq!(
        body["Source"],
        "arn:aws:sqs:us-east-1:000000000000:orders-queue"
    );
    assert_eq!(
        body["Target"],
        "arn:aws:sns:us-east-1:000000000000:orders-topic"
    );

    // 3. Stop Pipe
    let (status, body) = client
        .call_rest_json(Method::POST, "/v1/pipes/order-stream-pipe/stop", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["CurrentState"], "STOPPED");

    // 4. Start Pipe
    let (status, body) = client
        .call_rest_json(Method::POST, "/v1/pipes/order-stream-pipe/start", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["CurrentState"], "RUNNING");

    // 5. List Pipes
    let (status, body) = client.call_rest_json(Method::GET, "/v1/pipes", None).await;
    assert_eq!(status, StatusCode::OK);
    let pipes = body["Pipes"].as_array().unwrap();
    assert_eq!(pipes.len(), 1);
    assert_eq!(pipes[0]["Name"], "order-stream-pipe");

    // 6. Delete Pipe
    let (status, _) = client
        .call_rest_json(Method::DELETE, "/v1/pipes/order-stream-pipe", None)
        .await;
    assert_eq!(status, StatusCode::OK);

    // 7. Verify List is now empty
    let (status, body) = client.call_rest_json(Method::GET, "/v1/pipes", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["Pipes"].as_array().unwrap().len(), 0);
}
