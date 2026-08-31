use axum::http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_kinesis_stream_and_records_compat() {
    let client = RustStackTestClient::new();

    // 1. Create Stream
    let (status, _) = client
        .call_json(
            "Kinesis_20131202.CreateStream",
            json!({
                "StreamName": "order-events",
                "ShardCount": 1
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 2. Put Record
    let data_b64 = "eyJldmVudCI6ICJvcmRlcl9jcmVhdGVkIiwgImFtb3VudCI6IDk5fQ==";
    let (status, val) = client
        .call_json(
            "Kinesis_20131202.PutRecord",
            json!({
                "StreamName": "order-events",
                "Data": data_b64,
                "PartitionKey": "cust_101"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let shard_id = val["ShardId"].as_str().unwrap();

    // 3. Get Shard Iterator
    let (status, val) = client
        .call_json(
            "Kinesis_20131202.GetShardIterator",
            json!({
                "StreamName": "order-events",
                "ShardId": shard_id,
                "ShardIteratorType": "TRIM_HORIZON"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let iterator = val["ShardIterator"].as_str().unwrap();

    // 4. Get Records
    let (status, val) = client
        .call_json(
            "Kinesis_20131202.GetRecords",
            json!({
                "ShardIterator": iterator
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let records = val["Records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["PartitionKey"].as_str().unwrap(), "cust_101");
}
