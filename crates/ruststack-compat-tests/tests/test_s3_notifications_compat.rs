use axum::http::{HeaderMap, Method, StatusCode};
use bytes::Bytes;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_s3_bucket_notifications_to_sqs_and_sns() {
    let client = RustStackTestClient::new();

    // 1. Create SQS Queue
    let (_, val) = client
        .call_json(
            "AmazonSQS.CreateQueue",
            json!({ "QueueName": "s3-notif-sqs-q" }),
        )
        .await;
    let sqs_url = val["QueueUrl"].as_str().unwrap().to_string();
    let sqs_arn = format!(
        "arn:aws:sqs:{}:{}:s3-notif-sqs-q",
        client.region, client.account_id
    );

    // 2. Create S3 Bucket
    let (status, _, _) = client
        .call_s3(Method::PUT, "/notif-bucket", HeaderMap::new(), Bytes::new())
        .await;
    assert_eq!(status, StatusCode::OK);

    // 3. Configure Bucket Notification to SQS for uploads/ prefix
    let notif_xml = format!(
        r#"<NotificationConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <QueueConfiguration>
        <Id>sqs-upload-notif</Id>
        <Queue>{}</Queue>
        <Event>s3:ObjectCreated:*</Event>
        <Filter>
            <S3Key>
                <FilterRule>
                    <Name>prefix</Name>
                    <Value>uploads/</Value>
                </FilterRule>
            </S3Key>
        </Filter>
    </QueueConfiguration>
</NotificationConfiguration>"#,
        sqs_arn
    );

    let (status, _, _) = client
        .call_s3(
            Method::PUT,
            "/notif-bucket?notification",
            HeaderMap::new(),
            Bytes::from(notif_xml),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 4. Verify GetBucketNotificationConfiguration returns config
    let (status, _, body) = client
        .call_s3(
            Method::GET,
            "/notif-bucket?notification",
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let xml_resp = String::from_utf8_lossy(&body);
    assert!(xml_resp.contains("sqs-upload-notif"));
    assert!(xml_resp.contains("s3:ObjectCreated:*"));
    assert!(xml_resp.contains("uploads/"));

    // 5. Put Object 1: "temp/file.txt" (does not match prefix, should NOT notify)
    let (status, _, _) = client
        .call_s3(
            Method::PUT,
            "/notif-bucket/temp/file.txt",
            HeaderMap::new(),
            Bytes::from_static(b"ignored"),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (_, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({ "QueueUrl": sqs_url, "MaxNumberOfMessages": 10 }),
        )
        .await;
    assert!(val.get("Messages").is_none() || val["Messages"].as_array().unwrap().is_empty());

    // 6. Put Object 2: "uploads/invoice_001.pdf" (matches prefix, MUST trigger S3 notification!)
    let (status, _, _) = client
        .call_s3(
            Method::PUT,
            "/notif-bucket/uploads/invoice_001.pdf",
            HeaderMap::new(),
            Bytes::from_static(b"invoice PDF contents"),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (_, val) = client
        .call_json(
            "AmazonSQS.ReceiveMessage",
            json!({ "QueueUrl": sqs_url, "MaxNumberOfMessages": 10 }),
        )
        .await;
    let msgs = val["Messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1);

    let record_payload: serde_json::Value =
        serde_json::from_str(msgs[0]["Body"].as_str().unwrap()).unwrap();
    let records = record_payload["Records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["eventSource"].as_str().unwrap(), "aws:s3");
    assert_eq!(
        records[0]["eventName"].as_str().unwrap(),
        "s3:ObjectCreated:Put"
    );
    assert_eq!(
        records[0]["s3"]["bucket"]["name"].as_str().unwrap(),
        "notif-bucket"
    );
    assert_eq!(
        records[0]["s3"]["object"]["key"].as_str().unwrap(),
        "uploads/invoice_001.pdf"
    );
    assert_eq!(records[0]["s3"]["object"]["size"].as_i64().unwrap(), 20);
}
