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

const AWS_SHAPED_QUEUE_URL: &str =
    "https://sqs.us-west-2.amazonaws.com/123456789012/delayed-orders-queue";
const SAMPLE_MESSAGE_BODY: &str = r#"{"id":"11111111-2222-3333-4444-555555555555"}"#;
const SAMPLE_MESSAGE_BODY_MD5: &str = "e2d1f80f6b189b142ed39fa47ed4d199";

/// End-to-end JSON protocol lifecycle with queue-level DelaySeconds=1:
/// create queue, resolve via GetQueueUrl, send through a real-AWS-shaped
/// queue URL, long-poll ReceiveMessage (WaitTimeSeconds=5) so the delayed
/// message becomes visible mid-poll, then PurgeQueue + empty long-poll.
#[tokio::test]
async fn test_sqs_delayed_long_poll_lifecycle_on_json_protocol() {
    let client = RustStackTestClient::new();

    // CreateQueue with queue-level DelaySeconds
    let (status, val) = client
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({
                "QueueName": "delayed-orders-queue",
                "Attributes": { "DelaySeconds": "1" }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        val["QueueUrl"].as_str().unwrap(),
        "http://localhost:4566/000000000000/delayed-orders-queue"
    );

    // GetQueueUrl resolves the created queue
    let (status, val) = client
        .call_json(
            "AmazonSQS.GetQueueUrl",
            json!({ "QueueName": "delayed-orders-queue" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        val["QueueUrl"].as_str().unwrap(),
        "http://localhost:4566/000000000000/delayed-orders-queue"
    );

    // SendMessage through a real-AWS-shaped queue URL (queue-name
    // resolution must not care about the host/account segments)
    let (status, val) = client
        .call_json(
            "AmazonSQS.SendMessage",
            json!({ "QueueUrl": AWS_SHAPED_QUEUE_URL, "MessageBody": SAMPLE_MESSAGE_BODY }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(val["MessageId"].is_string());
    assert_eq!(
        val["MD5OfMessageBody"].as_str().unwrap(),
        SAMPLE_MESSAGE_BODY_MD5
    );

    // Long-poll receive started inside the 1s queue-level delay window:
    // the message must become visible mid-poll, not only on new sends.
    let (status, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({
                "QueueUrl": AWS_SHAPED_QUEUE_URL,
                "MaxNumberOfMessages": 1,
                "WaitTimeSeconds": 5
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let messages = val["Messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1, "delayed message must arrive mid-poll");
    assert_eq!(messages[0]["Body"].as_str().unwrap(), SAMPLE_MESSAGE_BODY);
    assert_eq!(
        messages[0]["MD5OfBody"].as_str().unwrap(),
        SAMPLE_MESSAGE_BODY_MD5
    );

    // PurgeQueue + empty long-poll
    let (status, _) = client
        .call_json(
            "AmazonSQS.PurgeQueue",
            json!({ "QueueUrl": AWS_SHAPED_QUEUE_URL }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({ "QueueUrl": AWS_SHAPED_QUEUE_URL, "WaitTimeSeconds": 1 }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(val["Messages"].as_array().unwrap().is_empty());
}

/// SendMessage carrying MessageAttributes must return SQS's canonical
/// MD5OfMessageAttributes (sorted names, length-prefixed name + DataType
/// + transport type + value), and the attributes must round-trip through
/// ReceiveMessage.
#[tokio::test]
async fn test_sqs_send_message_attributes_canonical_md5() {
    let client = RustStackTestClient::new();
    let (status, val) = client
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({ "QueueName": "attrs-queue" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let queue_url = val["QueueUrl"].as_str().unwrap();

    let (status, val) = client
        .call_json(
            "AmazonSQS.SendMessage",
            json!({
                "QueueUrl": queue_url,
                "MessageBody": SAMPLE_MESSAGE_BODY,
                "MessageAttributes": {
                    "traceId": {
                        "DataType": "String",
                        "StringValue": "11111111-2222-3333-4444-555555555555"
                    }
                }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    // Reference digest computed with an independent implementation of the
    // AWS algorithm (4-byte big-endian lengths, name+DataType+0x01+value).
    assert_eq!(
        val["MD5OfMessageAttributes"].as_str().unwrap(),
        "eb46854758ae9fa2188a8b4300fecb6a"
    );
    assert_eq!(
        val["MD5OfMessageBody"].as_str().unwrap(),
        SAMPLE_MESSAGE_BODY_MD5
    );

    let (status, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({ "QueueUrl": queue_url, "WaitTimeSeconds": 1 }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let messages = val["Messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0]["MessageAttributes"]["traceId"]["DataType"]
            .as_str()
            .unwrap(),
        "String"
    );
    assert_eq!(
        messages[0]["MessageAttributes"]["traceId"]["StringValue"]
            .as_str()
            .unwrap(),
        "11111111-2222-3333-4444-555555555555"
    );
}

/// SendMessageBatch entries with MessageAttributes (previously dropped by
/// the JSON handler) must keep their attributes and report the canonical
/// per-entry MD5OfMessageAttributes.
#[tokio::test]
async fn test_sqs_send_message_batch_keeps_attributes() {
    let client = RustStackTestClient::new();
    let (status, val) = client
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({ "QueueName": "batch-attrs-queue" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let queue_url = val["QueueUrl"].as_str().unwrap();

    let (status, val) = client
        .call_json(
            "AmazonSQS.SendMessageBatch",
            json!({
                "QueueUrl": queue_url,
                "Entries": [{
                    "Id": "entry-1",
                    "MessageBody": SAMPLE_MESSAGE_BODY,
                    "MessageAttributes": {
                        "a": { "DataType": "String", "StringValue": "one" },
                        "b": { "DataType": "String", "StringValue": "two" }
                    }
                }]
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let successful = val["Successful"].as_array().unwrap();
    assert_eq!(successful.len(), 1);
    assert_eq!(successful[0]["Id"].as_str().unwrap(), "entry-1");
    // Attributes sorted by name (a, b): 4-byte lengths, 0x01 transport.
    assert_eq!(
        successful[0]["MD5OfMessageAttributes"].as_str().unwrap(),
        "dbe5cf7779eae4bb2d740e3abd1670b1"
    );

    let (status, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({ "QueueUrl": queue_url, "WaitTimeSeconds": 1 }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let messages = val["Messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0]["MessageAttributes"]["a"]["StringValue"]
            .as_str()
            .unwrap(),
        "one"
    );
    assert_eq!(
        messages[0]["MessageAttributes"]["b"]["StringValue"]
            .as_str()
            .unwrap(),
        "two"
    );
}

/// Queue URLs must follow the requesting client's Host header (LocalStack
/// parity), so SDKs inside containers with non-localhost emulator hosts
/// keep the URLs CreateQueue/GetQueueUrl return usable.
#[tokio::test]
async fn test_sqs_queue_urls_follow_request_host() {
    let client = RustStackTestClient::new();

    let create_req = http::Request::builder()
        .method("POST")
        .uri("/")
        .header("host", "ruststack:4566")
        .header("x-amz-target", "AmazonSQS.CreateQueue")
        .header("content-type", "application/x-amz-json-1.0")
        .body(axum::body::Body::from(
            json!({ "QueueName": "host-queue" }).to_string(),
        ))
        .unwrap();
    let resp = client.send_request(create_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(resp.into_body())
        .await
        .unwrap()
        .to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        val["QueueUrl"].as_str().unwrap(),
        "http://ruststack:4566/000000000000/host-queue"
    );

    let get_req = http::Request::builder()
        .method("POST")
        .uri("/")
        .header("host", "ruststack:4566")
        .header("x-amz-target", "AmazonSQS.GetQueueUrl")
        .header("content-type", "application/x-amz-json-1.0")
        .body(axum::body::Body::from(
            json!({ "QueueName": "host-queue" }).to_string(),
        ))
        .unwrap();
    let resp = client.send_request(get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(resp.into_body())
        .await
        .unwrap()
        .to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        val["QueueUrl"].as_str().unwrap(),
        "http://ruststack:4566/000000000000/host-queue"
    );

    // A different host still addresses the same queue (name-based lookup)
    let (status, val) = client
        .call_json(
            "AmazonSQS.SendMessage",
            json!({
                "QueueUrl": "http://localhost:4566/000000000000/host-queue",
                "MessageBody": "cross-host"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(val["MessageId"].is_string());
}
