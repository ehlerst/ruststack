use axum::http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_sqs_json_protocol_and_dlq() {
    let client = RustStackTestClient::new();

    // 1. Create DLQ
    let (status, val) = client
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({ "QueueName": "compat-dlq" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let dlq_url = val["QueueUrl"].as_str().unwrap().to_string();

    let dlq_arn = format!(
        "arn:aws:sqs:{}:{}:compat-dlq",
        client.region, client.account_id
    );

    // 2. Create Main Queue with Redrive Policy (maxReceiveCount = 2)
    let redrive_policy = json!({
        "deadLetterTargetArn": dlq_arn,
        "maxReceiveCount": 2
    })
    .to_string();

    let (status, val) = client
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({
                "QueueName": "compat-main-queue",
                "Attributes": {
                    "RedrivePolicy": redrive_policy
                }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let main_queue_url = val["QueueUrl"].as_str().unwrap().to_string();

    // 3. Send Message
    let (status, val) = client
        .call_json(
            "AmazonSQS.SendMessage",
            json!({
                "QueueUrl": main_queue_url,
                "MessageBody": "Order processing payload #123"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(val["MessageId"].is_string());

    // 4. Receive Message (Attempt 1, VisibilityTimeout 0s so it becomes immediately available)
    let (status, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({
                "QueueUrl": main_queue_url,
                "MaxNumberOfMessages": 1,
                "VisibilityTimeout": 0
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(val["Messages"].as_array().unwrap().len(), 1);

    // 5. Receive Message (Attempt 2, receive_count reaches 2)
    let (status, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({
                "QueueUrl": main_queue_url,
                "MaxNumberOfMessages": 1,
                "VisibilityTimeout": 0
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(val["Messages"].as_array().unwrap().len(), 1);

    // 6. Receive Message (Attempt 3 on main queue triggers redrive expiration to DLQ)
    let (_, _) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({
                "QueueUrl": main_queue_url,
                "MaxNumberOfMessages": 1,
                "VisibilityTimeout": 0
            }),
        )
        .await;

    // 7. Receive from DLQ (Message must have been moved to DLQ)
    let (status, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({
                "QueueUrl": dlq_url,
                "MaxNumberOfMessages": 1
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let dlq_msgs = val["Messages"].as_array().unwrap();
    assert_eq!(dlq_msgs.len(), 1);
    assert_eq!(
        dlq_msgs[0]["Body"].as_str().unwrap(),
        "Order processing payload #123"
    );
}

#[tokio::test]
async fn test_sqs_query_protocol_lifecycle() {
    let client = RustStackTestClient::new();

    // 1. CreateQueue via Query protocol
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "CreateQueue"),
                ("QueueName", "query-proto-queue"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<QueueUrl>"));

    // 2. SendMessage via Query protocol
    let (status, body) = client
        .call_query(
            "/000000000000/query-proto-queue",
            &[
                ("Action", "SendMessage"),
                ("MessageBody", "hello-query-protocol"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<MessageId>"));

    // 3. ReceiveMessage via Query protocol
    let (status, body) = client
        .call_query(
            "/000000000000/query-proto-queue",
            &[("Action", "ReceiveMessage"), ("MaxNumberOfMessages", "1")],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<Body>hello-query-protocol</Body>"));
}
