use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_sqs::{handle_sqs_request, SqsEngine};
use serde_json::Value;
use std::sync::Arc;

fn setup_sqs() -> Arc<SqsEngine> {
    Arc::new(SqsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ))
}

#[tokio::test]
async fn test_sqs_query_protocol_lifecycle() {
    let engine = setup_sqs();

    // 1. Create Queue via Query protocol
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("Action=CreateQueue&QueueName=test-query-queue"))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        xml.contains("<QueueUrl>http://localhost:4566/000000000000/test-query-queue</QueueUrl>")
    );

    // 2. Send Message
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(
            "Action=SendMessage&QueueUrl=http://localhost:4566/000000000000/test-query-queue&MessageBody=HelloFromRustStack",
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    assert!(xml.contains("<MessageId>"));

    // 3. Receive Message
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(
            "Action=ReceiveMessage&QueueUrl=http://localhost:4566/000000000000/test-query-queue&MaxNumberOfMessages=1",
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    assert!(xml.contains("<Body>HelloFromRustStack</Body>"));
    let receipt_handle = xml
        .split("<ReceiptHandle>")
        .nth(1)
        .unwrap()
        .split("</ReceiptHandle>")
        .next()
        .unwrap();

    // 4. Delete Message
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "Action=DeleteMessage&QueueUrl=http://localhost:4566/000000000000/test-query-queue&ReceiptHandle={}",
            receipt_handle
        )))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. Receive again -> Empty
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(
            "Action=ReceiveMessage&QueueUrl=http://localhost:4566/000000000000/test-query-queue&MaxNumberOfMessages=1",
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    assert!(!xml.contains("<Body>"));
}

#[tokio::test]
async fn test_sqs_json_protocol_lifecycle() {
    let engine = setup_sqs();

    // 1. Create Queue via JSON 1.0
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.CreateQueue")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(r#"{"QueueName": "json-queue"}"#))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let queue_url = val["QueueUrl"].as_str().unwrap();

    // 2. Send Message via JSON
    let send_body = serde_json::json!({
        "QueueUrl": queue_url,
        "MessageBody": "{\"user_id\": 42, \"action\": \"login\"}",
        "MessageAttributes": {
            "source": {
                "DataType": "String",
                "StringValue": "rust-test"
            }
        }
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.SendMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(send_body.to_string()))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert!(val.get("MessageId").is_some());
    assert!(val.get("MD5OfMessageBody").is_some());

    // 3. Receive Message via JSON
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.ReceiveMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueUrl": queue_url,
                "MaxNumberOfMessages": 10
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let messages = val["Messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    let receipt = messages[0]["ReceiptHandle"].as_str().unwrap();
    assert_eq!(
        messages[0]["MessageAttributes"]["source"]["StringValue"]
            .as_str()
            .unwrap(),
        "rust-test"
    );

    // 4. Delete Message via JSON
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.DeleteMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueUrl": queue_url,
                "ReceiptHandle": receipt
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_sqs_fifo_queue() {
    let engine = setup_sqs();

    // Create FIFO Queue
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.CreateQueue")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(r#"{"QueueName": "orders.fifo", "Attributes": {"FifoQueue": "true", "ContentBasedDeduplication": "true"}}"#))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let queue_url = val["QueueUrl"].as_str().unwrap();

    // Send 1st message with dedup id
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.SendMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueUrl": queue_url,
                "MessageBody": "order_100",
                "MessageGroupId": "group-1",
                "MessageDeduplicationId": "dedup-order-100"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Send duplicate message
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.SendMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueUrl": queue_url,
                "MessageBody": "order_100",
                "MessageGroupId": "group-1",
                "MessageDeduplicationId": "dedup-order-100"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Receive -> only 1 message should be present
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.ReceiveMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueUrl": queue_url,
                "MaxNumberOfMessages": 10
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let messages = val["Messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
}

#[tokio::test]
async fn test_sqs_dead_letter_queue_redrive() {
    let engine = setup_sqs();

    // 1. Create DLQ
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.CreateQueue")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(r#"{"QueueName": "poison-dlq"}"#))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let dlq_url = val["QueueUrl"].as_str().unwrap();

    // 2. Create Source Queue with RedrivePolicy (maxReceiveCount = 2)
    let redrive_policy = serde_json::json!({
        "deadLetterTargetArn": "arn:aws:sqs:us-east-1:000000000000:poison-dlq",
        "maxReceiveCount": 2
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.CreateQueue")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueName": "source-work-queue",
                "Attributes": {
                    "VisibilityTimeout": "1",
                    "RedrivePolicy": redrive_policy.to_string()
                }
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let src_url = val["QueueUrl"].as_str().unwrap();

    // 3. Test ListDeadLetterSourceQueues
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.ListDeadLetterSourceQueues")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueUrl": dlq_url
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let src_list = val["queueUrls"].as_array().unwrap();
    assert_eq!(src_list.len(), 1);
    assert_eq!(src_list[0].as_str().unwrap(), src_url);

    // 4. Send Message to source queue
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.SendMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueUrl": src_url,
                "MessageBody": "poison_pill_payload"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. Receive #1 (Visibility Timeout = 1s, don't delete)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.ReceiveMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueUrl": src_url,
                "VisibilityTimeout": 1
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Wait for visibility timeout to expire
    tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;

    // 6. Receive #2 (receive_count becomes 2, visibility expires -> exceeds maxReceiveCount)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.ReceiveMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueUrl": src_url,
                "VisibilityTimeout": 1
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Wait for visibility timeout to expire again
    tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;

    // 7. Receive from Source Queue -> Should now be Empty!
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.ReceiveMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueUrl": src_url
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["Messages"].as_array().unwrap().len(), 0);

    // 8. Receive from DLQ -> Message should have been automatically redriven to DLQ!
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.ReceiveMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueUrl": dlq_url
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(engine.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let dlq_msgs = val["Messages"].as_array().unwrap();
    assert_eq!(dlq_msgs.len(), 1);
    assert_eq!(dlq_msgs[0]["Body"].as_str().unwrap(), "poison_pill_payload");
}
