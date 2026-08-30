use axum::http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_eventbridge_rule_pattern_matching_and_target_dispatch() {
    let client = RustStackTestClient::new();

    // 1. Create SQS target queue
    let (_, val) = client
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({ "QueueName": "eb-compat-target-q" }),
        )
        .await;
    let target_q_url = val["QueueUrl"].as_str().unwrap().to_string();

    let target_q_arn = format!(
        "arn:aws:sqs:{}:{}:eb-compat-target-q",
        client.region, client.account_id
    );

    // 2. PutRule with content pattern
    let pattern = json!({
        "source": ["payments.service"],
        "detail-type": ["PaymentAuthorized"],
        "detail": {
            "currency": ["USD"],
            "amount": [{ "prefix": "100" }]
        }
    })
    .to_string();

    let (status, val) = client
        .call_json(
            "AWSEvents.PutRule",
            json!({
                "Name": "high-value-usd-payments",
                "EventPattern": pattern,
                "State": "ENABLED"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(val["RuleArn"].is_string());

    // 3. PutTargets
    let (status, _) = client
        .call_json(
            "AWSEvents.PutTargets",
            json!({
                "Rule": "high-value-usd-payments",
                "Targets": [
                    {
                        "Id": "sqs-auditing",
                        "Arn": target_q_arn
                    }
                ]
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 4. PutEvents non-matching event (EUR payment)
    let (status, val) = client
        .call_json(
            "AWSEvents.PutEvents",
            json!({
                "Entries": [
                    {
                        "Source": "payments.service",
                        "DetailType": "PaymentAuthorized",
                        "Detail": "{\"currency\": \"EUR\", \"amount\": \"100.00\"}"
                    }
                ]
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(val["FailedEntryCount"].as_i64().unwrap(), 0);

    // Target queue should be empty
    let (_, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({ "QueueUrl": target_q_url, "MaxNumberOfMessages": 10 }),
        )
        .await;
    assert!(val.get("Messages").is_none() || val["Messages"].as_array().unwrap().is_empty());

    // 5. PutEvents matching event (USD $100.50 payment)
    let (status, _) = client
        .call_json(
            "AWSEvents.PutEvents",
            json!({
                "Entries": [
                    {
                        "Source": "payments.service",
                        "DetailType": "PaymentAuthorized",
                        "Detail": "{\"currency\": \"USD\", \"amount\": \"100.50\", \"auth_id\": \"auth_999\"}"
                    }
                ]
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Target queue must receive event
    let (_, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({ "QueueUrl": target_q_url, "MaxNumberOfMessages": 10 }),
        )
        .await;
    let msgs = val["Messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1);
    let event_json: serde_json::Value =
        serde_json::from_str(msgs[0]["Body"].as_str().unwrap()).unwrap();
    assert_eq!(event_json["source"].as_str().unwrap(), "payments.service");
    assert_eq!(
        event_json["detail-type"].as_str().unwrap(),
        "PaymentAuthorized"
    );
}
