use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_eventbridge::{handle_eventbridge_request, EventBridgeEngine};
use ruststack_sns::SnsEngine;
use ruststack_sqs::{handle_sqs_request, SqsEngine};
use serde_json::Value;
use std::sync::Arc;

fn setup_eventbridge() -> (Arc<EventBridgeEngine>, Arc<SqsEngine>, Arc<SnsEngine>) {
    let sqs = Arc::new(SqsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let sns = Arc::new(SnsEngine::new(
        sqs.clone(),
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let eb = Arc::new(EventBridgeEngine::new(
        sqs.clone(),
        sns.clone(),
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    (eb, sqs, sns)
}

#[tokio::test]
async fn test_eventbridge_bus_and_rule_lifecycle() {
    let (eb, _, _) = setup_eventbridge();

    // 1. Create Event Bus
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AWSEvents.CreateEventBus")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(r#"{"Name": "custom-bus"}"#))
        .unwrap();
    let resp = handle_eventbridge_request(eb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        val["EventBusArn"].as_str().unwrap(),
        "arn:aws:events:us-east-1:000000000000:event-bus/custom-bus"
    );

    // 2. Put Rule on custom bus
    let pattern = r#"{"source": ["payment.gateway"], "detail-type": ["PaymentCaptured"]}"#;
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AWSEvents.PutRule")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Name": "payment-rule",
                "EventBusName": "custom-bus",
                "EventPattern": pattern,
                "State": "ENABLED"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_eventbridge_request(eb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Describe Rule
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AWSEvents.DescribeRule")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Name": "payment-rule",
                "EventBusName": "custom-bus"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_eventbridge_request(eb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["Name"].as_str().unwrap(), "payment-rule");
    assert_eq!(val["State"].as_str().unwrap(), "ENABLED");
}

#[tokio::test]
async fn test_eventbridge_put_events_with_sqs_target() {
    let (eb, sqs, _) = setup_eventbridge();

    // 1. Create SQS Queue target
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.CreateQueue")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(r#"{"QueueName": "orders-processor-queue"}"#))
        .unwrap();
    let resp = handle_sqs_request(sqs.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let queue_url = val["QueueUrl"].as_str().unwrap();
    let queue_arn = "arn:aws:sqs:us-east-1:000000000000:orders-processor-queue";

    // 2. Put Rule matching source "shop.checkout"
    let pattern = r#"{"source": ["shop.checkout"], "detail": {"currency": ["USD"]}}"#;
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AWSEvents.PutRule")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Name": "usd-orders-rule",
                "EventPattern": pattern,
                "State": "ENABLED"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_eventbridge_request(eb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Put Target (pointing to the SQS queue ARN)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AWSEvents.PutTargets")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Rule": "usd-orders-rule",
                "Targets": [
                    {
                        "Id": "sqs-target-1",
                        "Arn": queue_arn
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_eventbridge_request(eb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. PutEvents #1: Non-matching event (currency: "EUR")
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AWSEvents.PutEvents")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Entries": [
                    {
                        "Source": "shop.checkout",
                        "DetailType": "OrderCompleted",
                        "Detail": "{\"order_id\": 101, \"currency\": \"EUR\"}"
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_eventbridge_request(eb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. PutEvents #2: Matching event (currency: "USD")
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AWSEvents.PutEvents")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            serde_json::json!({
                "Entries": [
                    {
                        "Source": "shop.checkout",
                        "DetailType": "OrderCompleted",
                        "Detail": "{\"order_id\": 102, \"currency\": \"USD\"}"
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_eventbridge_request(eb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["FailedEntryCount"].as_u64().unwrap(), 0);
    assert!(val["Entries"][0]["EventId"].is_string());

    // 6. Receive from SQS Queue -> Exactly 1 message should be present!
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
    let resp = handle_sqs_request(sqs.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let messages = val["Messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);

    // 7. Verify CloudWatch Event JSON schema inside SQS message body
    let msg_body = messages[0]["Body"].as_str().unwrap();
    let event_obj: Value = serde_json::from_str(msg_body).unwrap();
    assert_eq!(event_obj["version"].as_str().unwrap(), "0");
    assert_eq!(event_obj["source"].as_str().unwrap(), "shop.checkout");
    assert_eq!(event_obj["detail-type"].as_str().unwrap(), "OrderCompleted");
    assert_eq!(event_obj["detail"]["order_id"].as_i64().unwrap(), 102);
    assert_eq!(event_obj["detail"]["currency"].as_str().unwrap(), "USD");
}
