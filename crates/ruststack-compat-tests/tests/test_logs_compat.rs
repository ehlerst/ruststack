use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_cloudwatch_logs_protocol_lifecycle() {
    let client = RustStackTestClient::new();

    // 1. CreateLogGroup
    let (status, _) = client
        .call_json(
            "Logs_20140328.CreateLogGroup",
            json!({
                "logGroupName": "/aws/lambda/checkout-service"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 2. DescribeLogGroups
    let (status, val) = client
        .call_json("Logs_20140328.DescribeLogGroups", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    let groups = val["logGroups"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["logGroupName"], "/aws/lambda/checkout-service");

    // 3. CreateLogStream
    let (status, _) = client
        .call_json(
            "Logs_20140328.CreateLogStream",
            json!({
                "logGroupName": "/aws/lambda/checkout-service",
                "logStreamName": "2026/08/30/stream-1"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 4. DescribeLogStreams
    let (status, val) = client
        .call_json(
            "Logs_20140328.DescribeLogStreams",
            json!({
                "logGroupName": "/aws/lambda/checkout-service"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let streams = val["logStreams"].as_array().unwrap();
    assert_eq!(streams.len(), 1);
    assert_eq!(streams[0]["logStreamName"], "2026/08/30/stream-1");

    // 5. PutLogEvents
    let (status, val) = client
        .call_json(
            "Logs_20140328.PutLogEvents",
            json!({
                "logGroupName": "/aws/lambda/checkout-service",
                "logStreamName": "2026/08/30/stream-1",
                "logEvents": [
                    { "timestamp": 1725000000, "message": "Order #123 placed" },
                    { "timestamp": 1725000010, "message": "Payment verified" }
                ]
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(val.get("nextSequenceToken").is_some());

    // 6. FilterLogEvents
    let (status, val) = client
        .call_json(
            "Logs_20140328.FilterLogEvents",
            json!({
                "logGroupName": "/aws/lambda/checkout-service",
                "filterPattern": "Payment"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let events = val["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["message"], "Payment verified");

    // 7. DeleteLogGroup
    let (status, _) = client
        .call_json(
            "Logs_20140328.DeleteLogGroup",
            json!({
                "logGroupName": "/aws/lambda/checkout-service"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}
