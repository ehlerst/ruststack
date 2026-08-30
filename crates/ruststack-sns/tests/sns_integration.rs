use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_sns::{handle_sns_request, SnsEngine};
use ruststack_sqs::{handle_sqs_request, SqsEngine};
use serde_json::Value;
use std::sync::Arc;

fn setup_sns_and_sqs() -> (Arc<SnsEngine>, Arc<SqsEngine>) {
    let sqs = Arc::new(SqsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let sns = Arc::new(SnsEngine::new(
        sqs.clone(),
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    (sns, sqs)
}

#[tokio::test]
async fn test_sns_topic_lifecycle() {
    let (sns, _) = setup_sns_and_sqs();

    // 1. Create Topic via Query protocol
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("Action=CreateTopic&Name=my-alerts"))
        .unwrap();
    let resp = handle_sns_request(sns.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    assert!(xml.contains("<TopicArn>arn:aws:sns:us-east-1:000000000000:my-alerts</TopicArn>"));

    // 2. List Topics
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("Action=ListTopics"))
        .unwrap();
    let resp = handle_sns_request(sns.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    assert!(xml.contains("<TopicArn>arn:aws:sns:us-east-1:000000000000:my-alerts</TopicArn>"));

    // 3. Delete Topic
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(
            "Action=DeleteTopic&TopicArn=arn:aws:sns:us-east-1:000000000000:my-alerts",
        ))
        .unwrap();
    let resp = handle_sns_request(sns.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_sns_to_sqs_fanout_json() {
    let (sns, sqs) = setup_sns_and_sqs();

    // 1. Create SQS Queue
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.CreateQueue")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(r#"{"QueueName": "subscriber-queue"}"#))
        .unwrap();
    let resp = handle_sqs_request(sqs.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let queue_url = val["QueueUrl"].as_str().unwrap();

    // 2. Create SNS Topic
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSNS.CreateTopic")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(r#"{"Name": "events-topic"}"#))
        .unwrap();
    let resp = handle_sns_request(sns.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let topic_arn = val["TopicArn"].as_str().unwrap();

    // 3. Subscribe SQS to SNS Topic
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSNS.Subscribe")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TopicArn": topic_arn,
                "Protocol": "sqs",
                "Endpoint": queue_url
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sns_request(sns.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. Publish Message to SNS
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSNS.Publish")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TopicArn": topic_arn,
                "Message": "{\"order_id\": 999, \"status\": \"shipped\"}",
                "Subject": "Order Update"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sns_request(sns.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. Receive Message from SQS Queue (Verifying Fanout Envelope!)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.ReceiveMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "QueueUrl": queue_url,
                "MaxNumberOfMessages": 1
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_sqs_request(sqs.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let messages = val["Messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);

    // Verify SNS JSON Envelope structure
    let msg_body = messages[0]["Body"].as_str().unwrap();
    let env: Value = serde_json::from_str(msg_body).unwrap();
    assert_eq!(env["Type"].as_str().unwrap(), "Notification");
    assert_eq!(env["TopicArn"].as_str().unwrap(), topic_arn);
    assert_eq!(env["Subject"].as_str().unwrap(), "Order Update");
    assert_eq!(
        env["Message"].as_str().unwrap(),
        "{\"order_id\": 999, \"status\": \"shipped\"}"
    );
}

#[tokio::test]
async fn test_sns_raw_message_delivery_and_filter_policy() {
    let (sns, sqs) = setup_sns_and_sqs();

    // Create 2 queues: queue_eu and queue_us
    let q_eu = sqs.create_queue("queue-eu", None).unwrap();
    let q_us = sqs.create_queue("queue-us", None).unwrap();

    let topic_arn = sns.create_topic("regional-events", None).unwrap();

    // Subscribe EU with filter policy: region == ["eu-central-1"] and RawMessageDelivery
    let mut eu_attrs = std::collections::HashMap::new();
    eu_attrs.insert("RawMessageDelivery".to_string(), "true".to_string());
    eu_attrs.insert(
        "FilterPolicy".to_string(),
        r#"{"region": ["eu-central-1"]}"#.to_string(),
    );
    sns.subscribe(&topic_arn, "sqs", &q_eu, Some(eu_attrs))
        .unwrap();

    // Subscribe US with filter policy: region == ["us-east-1"] and RawMessageDelivery
    let mut us_attrs = std::collections::HashMap::new();
    us_attrs.insert("RawMessageDelivery".to_string(), "true".to_string());
    us_attrs.insert(
        "FilterPolicy".to_string(),
        r#"{"region": ["us-east-1"]}"#.to_string(),
    );
    sns.subscribe(&topic_arn, "sqs", &q_us, Some(us_attrs))
        .unwrap();

    // Publish EU event
    let mut eu_msg_attrs = std::collections::HashMap::new();
    eu_msg_attrs.insert(
        "region".to_string(),
        ruststack_sns::MessageAttributeValue {
            data_type: "String".to_string(),
            string_value: Some("eu-central-1".to_string()),
            binary_value: None,
        },
    );
    sns.publish(
        &topic_arn,
        "EU_RAW_DATA".to_string(),
        None,
        Some(eu_msg_attrs),
        None,
        None,
    )
    .unwrap();

    // Receive from EU queue -> should receive raw payload "EU_RAW_DATA"
    let eu_msgs = sqs.receive_message(&q_eu, 10, None, None).await.unwrap();
    assert_eq!(eu_msgs.len(), 1);
    assert_eq!(eu_msgs[0].body, "EU_RAW_DATA");

    // Receive from US queue -> should be empty (filtered out)
    let us_msgs = sqs.receive_message(&q_us, 10, None, None).await.unwrap();
    assert_eq!(us_msgs.len(), 0);
}
