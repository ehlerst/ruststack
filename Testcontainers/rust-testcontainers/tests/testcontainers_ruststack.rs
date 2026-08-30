use ruststack_testcontainers_example::ruststack_image;
use serde_json::json;
use testcontainers::runners::AsyncRunner;

#[tokio::test]
#[ignore = "requires running docker daemon and pulled ehlers320/ruststack:latest"]
async fn test_ruststack_container_s3_sqs_dynamodb() {
    let image = ruststack_image();
    let container = image.start().await.expect("failed to start ruststack container");
    let host_port = container
        .get_host_port_ipv4(4566)
        .await
        .expect("failed to get host port");

    let base_url = format!("http://127.0.0.1:{}", host_port);
    let http = reqwest::Client::new();

    // 1. Health check
    let resp = http
        .get(format!("{}/_ruststack/health", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // 2. S3 CreateBucket & PutObject
    let resp = http
        .put(format!("{}/container-test-bucket", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = http
        .put(format!("{}/container-test-bucket/test.txt", base_url))
        .body("hello from rust testcontainers")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = http
        .get(format!("{}/container-test-bucket/test.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let text = resp.text().await.unwrap();
    assert_eq!(text, "hello from rust testcontainers");

    // 3. SQS CreateQueue & Send/Receive
    let resp = http
        .post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSQS.CreateQueue")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({ "QueueName": "container-sqs-q" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let val: serde_json::Value = resp.json().await.unwrap();
    let q_url = val["QueueUrl"].as_str().unwrap().to_string();

    let resp = http
        .post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSQS.SendMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "QueueUrl": q_url,
            "MessageBody": "container-msg-123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = http
        .post(format!("{}/", base_url))
        .header("x-amz-target", "AmazonSQS.ReceiveMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "QueueUrl": q_url,
            "MaxNumberOfMessages": 1
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let val: serde_json::Value = resp.json().await.unwrap();
    let msgs = val["Messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["Body"].as_str().unwrap(), "container-msg-123");

    // 4. DynamoDB CreateTable & PutItem
    let resp = http
        .post(format!("{}/", base_url))
        .header("x-amz-target", "DynamoDB_20120810.CreateTable")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "TableName": "ContainerUsers",
            "KeySchema": [{ "AttributeName": "id", "KeyType": "HASH" }],
            "AttributeDefinitions": [{ "AttributeName": "id", "AttributeType": "S" }],
            "BillingMode": "PAY_PER_REQUEST"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = http
        .post(format!("{}/", base_url))
        .header("x-amz-target", "DynamoDB_20120810.PutItem")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "TableName": "ContainerUsers",
            "Item": {
                "id": { "S": "u-456" },
                "name": { "S": "Alice Rust Testcontainers" }
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = http
        .post(format!("{}/", base_url))
        .header("x-amz-target", "DynamoDB_20120810.GetItem")
        .header("content-type", "application/x-amz-json-1.0")
        .json(&json!({
            "TableName": "ContainerUsers",
            "Key": { "id": { "S": "u-456" } }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let val: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        val["Item"]["name"]["S"].as_str().unwrap(),
        "Alice Rust Testcontainers"
    );
}
