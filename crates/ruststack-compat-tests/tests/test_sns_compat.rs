use axum::http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_sns_to_sqs_fanout_and_filter_policy() {
    let client = RustStackTestClient::new();

    // 1. Create SNS Topic
    let (status, val) = client
        .call_json("AmazonSNS.CreateTopic", json!({ "Name": "orders-events" }))
        .await;
    assert_eq!(status, StatusCode::OK);
    let topic_arn = val["TopicArn"].as_str().unwrap().to_string();

    // 2. Create 2 SQS Queues (Orders Service & Analytics Service)
    let (_, val) = client
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({ "QueueName": "orders-svc-q" }),
        )
        .await;
    let orders_q_url = val["QueueUrl"].as_str().unwrap().to_string();

    let (_, val) = client
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({ "QueueName": "analytics-svc-q" }),
        )
        .await;
    let analytics_q_url = val["QueueUrl"].as_str().unwrap().to_string();

    // 3. Subscribe orders-svc-q with FilterPolicy: event_type = ["order_completed"]
    let filter_policy = json!({ "event_type": ["order_completed"] }).to_string();
    let (status, val) = client
        .call_json(
            "AmazonSNS.Subscribe",
            json!({
                "TopicArn": topic_arn,
                "Protocol": "sqs",
                "Endpoint": orders_q_url,
                "Attributes": {
                    "RawMessageDelivery": "true",
                    "FilterPolicy": filter_policy
                }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(val["SubscriptionArn"].is_string());

    // 4. Subscribe analytics-svc-q (no filter, receives all events)
    let (status, _) = client
        .call_json(
            "AmazonSNS.Subscribe",
            json!({
                "TopicArn": topic_arn,
                "Protocol": "sqs",
                "Endpoint": analytics_q_url,
                "Attributes": {
                    "RawMessageDelivery": "false"
                }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 5. Publish Event 1: event_type = "order_placed" (Orders queue filters out, Analytics queue receives)
    let (status, _) = client
        .call_json(
            "AmazonSNS.Publish",
            json!({
                "TopicArn": topic_arn,
                "Message": "{\"orderId\": \"ord_1\", \"status\": \"placed\"}",
                "MessageAttributes": {
                    "event_type": {
                        "DataType": "String",
                        "StringValue": "order_placed"
                    }
                }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Check orders queue (should be empty)
    let (_, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({ "QueueUrl": orders_q_url, "MaxNumberOfMessages": 10 }),
        )
        .await;
    assert!(val.get("Messages").is_none() || val["Messages"].as_array().unwrap().is_empty());

    // Check analytics queue (should receive JSON notification envelope)
    let (_, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({ "QueueUrl": analytics_q_url, "MaxNumberOfMessages": 10 }),
        )
        .await;
    let msgs = val["Messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1);
    let body_json: serde_json::Value =
        serde_json::from_str(msgs[0]["Body"].as_str().unwrap()).unwrap();
    assert_eq!(body_json["Type"].as_str().unwrap(), "Notification");
    assert!(body_json["Message"].as_str().unwrap().contains("ord_1"));

    // 6. Publish Event 2: event_type = "order_completed" (Both receive!)
    let (status, _) = client
        .call_json(
            "AmazonSNS.Publish",
            json!({
                "TopicArn": topic_arn,
                "Message": "{\"orderId\": \"ord_1\", \"status\": \"completed\"}",
                "MessageAttributes": {
                    "event_type": {
                        "DataType": "String",
                        "StringValue": "order_completed"
                    }
                }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Orders queue receives raw payload
    let (_, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({ "QueueUrl": orders_q_url, "MaxNumberOfMessages": 10 }),
        )
        .await;
    let orders_msgs = val["Messages"].as_array().unwrap();
    assert_eq!(orders_msgs.len(), 1);
    assert_eq!(
        orders_msgs[0]["Body"].as_str().unwrap(),
        "{\"orderId\": \"ord_1\", \"status\": \"completed\"}"
    );
}
