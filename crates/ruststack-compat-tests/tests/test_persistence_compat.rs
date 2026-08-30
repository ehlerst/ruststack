use axum::http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_auto_disk_persistence_cycle() {
    let temp_dir = tempfile::tempdir().unwrap();
    let state_file = temp_dir.path().join("state.json");
    let state_file_str = state_file.to_str().unwrap().to_string();

    // 1. First Cluster Session
    let client1 = RustStackTestClient::new();

    // Create S3 bucket
    let (s3_status, _, _) = client1
        .call_s3(
            axum::http::Method::PUT,
            "/persist-test-bucket",
            axum::http::HeaderMap::new(),
            bytes::Bytes::new(),
        )
        .await;
    assert_eq!(s3_status, StatusCode::OK);

    // Create DynamoDB table
    let (ddb_status, _) = client1
        .call_json(
            "DynamoDB_20120810.CreateTable",
            json!({
                "TableName": "PersistUsers",
                "KeySchema": [{ "AttributeName": "id", "KeyType": "HASH" }],
                "AttributeDefinitions": [{ "AttributeName": "id", "AttributeType": "S" }],
                "BillingMode": "PAY_PER_REQUEST"
            }),
        )
        .await;
    assert_eq!(ddb_status, StatusCode::OK);

    // Create SQS queue
    let (sqs_status, _) = client1
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({ "QueueName": "persist-tasks" }),
        )
        .await;
    assert_eq!(sqs_status, StatusCode::OK);

    // Create KMS key
    let (kms_status, kms_val) = client1
        .call_json(
            "TrentService.CreateKey",
            json!({
                "Description": "Persist Master Key",
                "KeyUsage": "ENCRYPT_DECRYPT"
            }),
        )
        .await;
    assert_eq!(kms_status, StatusCode::OK);
    let key_id = kms_val["KeyMetadata"]["KeyId"].as_str().unwrap().to_string();

    // 2. Dump State to Disk
    let dump_req = axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/_ruststack/state/dump")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            json!({ "file_path": state_file_str }).to_string(),
        ))
        .unwrap();
    let dump_resp = client1.send_request(dump_req).await;
    assert_eq!(dump_resp.status(), StatusCode::OK);
    assert!(state_file.exists());

    // 3. Second Cluster Session (Simulated Restart)
    let client2 = RustStackTestClient::new();

    // Verify client2 is initially empty of user resources
    let (head_status, _, _) = client2
        .call_s3(
            axum::http::Method::HEAD,
            "/persist-test-bucket",
            axum::http::HeaderMap::new(),
            bytes::Bytes::new(),
        )
        .await;
    assert_eq!(head_status, StatusCode::NOT_FOUND);

    // Load State from Disk
    let load_req = axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/_ruststack/state/load")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            json!({ "file_path": state_file_str }).to_string(),
        ))
        .unwrap();
    let load_resp = client2.send_request(load_req).await;
    assert_eq!(load_resp.status(), StatusCode::OK);

    // 4. Verify all resources restored in client2
    // S3 bucket exists
    let (head_status2, _, _) = client2
        .call_s3(
            axum::http::Method::HEAD,
            "/persist-test-bucket",
            axum::http::HeaderMap::new(),
            bytes::Bytes::new(),
        )
        .await;
    assert_eq!(head_status2, StatusCode::OK);

    // DynamoDB table exists
    let (ddb_desc_status, ddb_desc_val) = client2
        .call_json(
            "DynamoDB_20120810.DescribeTable",
            json!({ "TableName": "PersistUsers" }),
        )
        .await;
    assert_eq!(ddb_desc_status, StatusCode::OK);
    assert_eq!(ddb_desc_val["Table"]["TableName"], "PersistUsers");

    // SQS queue exists
    let (sqs_url_status, sqs_url_val) = client2
        .call_json(
            "AmazonSQS.GetQueueUrl",
            json!({ "QueueName": "persist-tasks" }),
        )
        .await;
    assert_eq!(sqs_url_status, StatusCode::OK);
    assert!(sqs_url_val["QueueUrl"].as_str().unwrap().contains("persist-tasks"));

    // KMS key exists
    let (kms_desc_status, kms_desc_val) = client2
        .call_json(
            "TrentService.DescribeKey",
            json!({ "KeyId": key_id }),
        )
        .await;
    assert_eq!(kms_desc_status, StatusCode::OK);
    assert_eq!(kms_desc_val["KeyMetadata"]["Description"], "Persist Master Key");
}
