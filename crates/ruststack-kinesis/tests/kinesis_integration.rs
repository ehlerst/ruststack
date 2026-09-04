use axum::extract::State;
use axum::http::HeaderMap;
use base64::Engine;
use bytes::Bytes;
use ruststack_kinesis::handle_kinesis_request;
use ruststack_kinesis::state::KinesisState;
use ruststack_kinesis::types::*;

#[tokio::test]
async fn test_stream_lifecycle() {
    let state = KinesisState::new("000000000000".to_string(), "us-east-1".to_string());

    // 1. Create stream
    state
        .create_stream(CreateStreamRequest {
            stream_name: "test-stream".to_string(),
            shard_count: Some(2),
            stream_mode_details: Some(StreamModeDetails {
                stream_mode: StreamMode::Provisioned,
            }),
        })
        .expect("create stream");

    // 2. Describe stream
    let desc = state
        .describe_stream(DescribeStreamRequest {
            stream_name: Some("test-stream".to_string()),
            ..Default::default()
        })
        .expect("describe stream");

    assert_eq!(desc.stream_name, "test-stream");
    assert_eq!(desc.stream_status, StreamStatus::Active);
    assert_eq!(desc.shards.len(), 2);
    assert_eq!(desc.shards[0].shard_id, "shardId-000000000000");
    assert_eq!(desc.shards[1].shard_id, "shardId-000000000001");

    // 3. Describe stream summary
    let summary = state
        .describe_stream_summary(DescribeStreamSummaryRequest {
            stream_name: Some("test-stream".to_string()),
            ..Default::default()
        })
        .expect("describe stream summary");
    assert_eq!(summary.stream_name, "test-stream");
    assert_eq!(summary.open_shard_count, 2);

    // 4. List streams
    let list = state
        .list_streams(ListStreamsRequest::default())
        .expect("list streams");
    assert_eq!(list.stream_names.len(), 1);
    assert_eq!(list.stream_names[0], "test-stream");

    // 5. Tags
    let mut tags = std::collections::HashMap::new();
    tags.insert("Environment".to_string(), "Production".to_string());
    tags.insert("Project".to_string(), "RustStack".to_string());

    state
        .add_tags_to_stream(AddTagsToStreamRequest {
            stream_name: Some("test-stream".to_string()),
            stream_arn: None,
            tags,
        })
        .expect("add tags");

    let tags_res = state
        .list_tags_for_stream(ListTagsForStreamRequest {
            stream_name: Some("test-stream".to_string()),
            ..Default::default()
        })
        .expect("list tags");
    assert_eq!(tags_res.tags.len(), 2);

    state
        .remove_tags_from_stream(RemoveTagsFromStreamRequest {
            stream_name: Some("test-stream".to_string()),
            stream_arn: None,
            tag_keys: vec!["Project".to_string()],
        })
        .expect("remove tags");

    let tags_after = state
        .list_tags_for_stream(ListTagsForStreamRequest {
            stream_name: Some("test-stream".to_string()),
            ..Default::default()
        })
        .expect("list tags after remove");
    assert_eq!(tags_after.tags.len(), 1);
    assert_eq!(tags_after.tags[0].key, "Environment");

    // 6. Delete stream
    state
        .delete_stream(DeleteStreamRequest {
            stream_name: Some("test-stream".to_string()),
            ..Default::default()
        })
        .expect("delete stream");

    let list_after = state
        .list_streams(ListStreamsRequest::default())
        .expect("list streams after delete");
    assert_eq!(list_after.stream_names.len(), 0);
}

#[tokio::test]
async fn test_record_ingestion_and_reading() {
    let state = KinesisState::new("000000000000".to_string(), "us-east-1".to_string());

    state
        .create_stream(CreateStreamRequest {
            stream_name: "orders-stream".to_string(),
            shard_count: Some(1),
            ..Default::default()
        })
        .expect("create stream");

    // 1. Put single record
    let data1 = base64::engine::general_purpose::STANDARD.encode("Order #1001");
    let put_res1 = state
        .put_record(PutRecordRequest {
            stream_name: Some("orders-stream".to_string()),
            stream_arn: None,
            data: data1.clone(),
            partition_key: "partition-1".to_string(),
            explicit_hash_key: None,
            sequence_number_for_ordering: None,
        })
        .expect("put record 1");
    assert_eq!(put_res1.shard_id, "shardId-000000000000");
    assert!(!put_res1.sequence_number.is_empty());

    // 2. Put batch of records
    let data2 = base64::engine::general_purpose::STANDARD.encode("Order #1002");
    let data3 = base64::engine::general_purpose::STANDARD.encode("Order #1003");
    let batch_res = state
        .put_records(PutRecordsRequest {
            stream_name: Some("orders-stream".to_string()),
            stream_arn: None,
            records: vec![
                PutRecordsRequestEntry {
                    data: data2.clone(),
                    partition_key: "partition-1".to_string(),
                    explicit_hash_key: None,
                },
                PutRecordsRequestEntry {
                    data: data3.clone(),
                    partition_key: "partition-1".to_string(),
                    explicit_hash_key: None,
                },
            ],
        })
        .expect("put records batch");
    assert_eq!(batch_res.failed_record_count, 0);
    assert_eq!(batch_res.records.len(), 2);

    // 3. Get Shard Iterator (TRIM_HORIZON)
    let iter_res = state
        .get_shard_iterator(GetShardIteratorRequest {
            stream_name: Some("orders-stream".to_string()),
            stream_arn: None,
            shard_id: "shardId-000000000000".to_string(),
            shard_iterator_type: ShardIteratorType::TrimHorizon,
            starting_sequence_number: None,
            timestamp: None,
        })
        .expect("get shard iterator");
    let iterator = iter_res.shard_iterator.expect("has iterator");

    // 4. Get records
    let get_res = state
        .get_records(GetRecordsRequest {
            shard_iterator: iterator,
            limit: Some(10),
            stream_arn: None,
        })
        .expect("get records");
    assert_eq!(get_res.records.len(), 3);
    assert_eq!(get_res.records[0].data, data1);
    assert_eq!(get_res.records[1].data, data2);
    assert_eq!(get_res.records[2].data, data3);

    // 5. Read next (should be empty now)
    let next_iter = get_res.next_shard_iterator.expect("next iterator");
    let get_empty = state
        .get_records(GetRecordsRequest {
            shard_iterator: next_iter.clone(),
            limit: Some(10),
            stream_arn: None,
        })
        .expect("get empty records");
    assert_eq!(get_empty.records.len(), 0);

    // 6. Put record and read via next_iter
    let data4 = base64::engine::general_purpose::STANDARD.encode("Order #1004");
    state
        .put_record(PutRecordRequest {
            stream_name: Some("orders-stream".to_string()),
            stream_arn: None,
            data: data4.clone(),
            partition_key: "partition-1".to_string(),
            explicit_hash_key: None,
            sequence_number_for_ordering: None,
        })
        .expect("put record 4");

    let get_new = state
        .get_records(GetRecordsRequest {
            shard_iterator: next_iter,
            limit: Some(10),
            stream_arn: None,
        })
        .expect("get new records");
    assert_eq!(get_new.records.len(), 1);
    assert_eq!(get_new.records[0].data, data4);
}

#[tokio::test]
async fn test_shard_iterator_types() {
    let state = KinesisState::new("000000000000".to_string(), "us-east-1".to_string());

    state
        .create_stream(CreateStreamRequest {
            stream_name: "events-stream".to_string(),
            shard_count: Some(1),
            ..Default::default()
        })
        .unwrap();

    let d1 = base64::engine::general_purpose::STANDARD.encode("event-1");
    let d2 = base64::engine::general_purpose::STANDARD.encode("event-2");
    let d3 = base64::engine::general_purpose::STANDARD.encode("event-3");

    let r1 = state
        .put_record(PutRecordRequest {
            stream_name: Some("events-stream".to_string()),
            data: d1.clone(),
            partition_key: "pk".to_string(),
            ..Default::default()
        })
        .unwrap();

    // LATEST iterator created now
    let latest_iter = state
        .get_shard_iterator(GetShardIteratorRequest {
            stream_name: Some("events-stream".to_string()),
            shard_id: "shardId-000000000000".to_string(),
            shard_iterator_type: ShardIteratorType::Latest,
            ..Default::default()
        })
        .unwrap()
        .shard_iterator
        .unwrap();

    let r2 = state
        .put_record(PutRecordRequest {
            stream_name: Some("events-stream".to_string()),
            data: d2.clone(),
            partition_key: "pk".to_string(),
            ..Default::default()
        })
        .unwrap();

    let _r3 = state
        .put_record(PutRecordRequest {
            stream_name: Some("events-stream".to_string()),
            data: d3.clone(),
            partition_key: "pk".to_string(),
            ..Default::default()
        })
        .unwrap();

    // LATEST should only see event-2 and event-3
    let latest_records = state
        .get_records(GetRecordsRequest {
            shard_iterator: latest_iter,
            limit: None,
            stream_arn: None,
        })
        .unwrap();
    assert_eq!(latest_records.records.len(), 2);
    assert_eq!(latest_records.records[0].data, d2);
    assert_eq!(latest_records.records[1].data, d3);

    // AT_SEQUENCE_NUMBER at r2
    let at_seq_iter = state
        .get_shard_iterator(GetShardIteratorRequest {
            stream_name: Some("events-stream".to_string()),
            shard_id: "shardId-000000000000".to_string(),
            shard_iterator_type: ShardIteratorType::AtSequenceNumber,
            starting_sequence_number: Some(r2.sequence_number.clone()),
            ..Default::default()
        })
        .unwrap()
        .shard_iterator
        .unwrap();

    let at_records = state
        .get_records(GetRecordsRequest {
            shard_iterator: at_seq_iter,
            limit: None,
            stream_arn: None,
        })
        .unwrap();
    assert_eq!(at_records.records.len(), 2);
    assert_eq!(at_records.records[0].data, d2);

    // AFTER_SEQUENCE_NUMBER after r1
    let after_seq_iter = state
        .get_shard_iterator(GetShardIteratorRequest {
            stream_name: Some("events-stream".to_string()),
            shard_id: "shardId-000000000000".to_string(),
            shard_iterator_type: ShardIteratorType::AfterSequenceNumber,
            starting_sequence_number: Some(r1.sequence_number.clone()),
            ..Default::default()
        })
        .unwrap()
        .shard_iterator
        .unwrap();

    let after_records = state
        .get_records(GetRecordsRequest {
            shard_iterator: after_seq_iter,
            limit: None,
            stream_arn: None,
        })
        .unwrap();
    assert_eq!(after_records.records.len(), 2);
    assert_eq!(after_records.records[0].data, d2);
}

#[tokio::test]
async fn test_snapshot_and_reset() {
    let state = KinesisState::new("000000000000".to_string(), "us-east-1".to_string());

    state
        .create_stream(CreateStreamRequest {
            stream_name: "snap-stream".to_string(),
            shard_count: Some(1),
            ..Default::default()
        })
        .unwrap();

    let data = base64::engine::general_purpose::STANDARD.encode("persistent-data");
    state
        .put_record(PutRecordRequest {
            stream_name: Some("snap-stream".to_string()),
            data: data.clone(),
            partition_key: "pk".to_string(),
            ..Default::default()
        })
        .unwrap();

    // Export snapshot
    let snap = state.export_snapshot();

    // Import into new state
    let new_state = KinesisState::new("000000000000".to_string(), "us-east-1".to_string());
    new_state.import_snapshot(snap);

    let iter = new_state
        .get_shard_iterator(GetShardIteratorRequest {
            stream_name: Some("snap-stream".to_string()),
            shard_id: "shardId-000000000000".to_string(),
            shard_iterator_type: ShardIteratorType::TrimHorizon,
            ..Default::default()
        })
        .unwrap()
        .shard_iterator
        .unwrap();

    let records = new_state
        .get_records(GetRecordsRequest {
            shard_iterator: iter,
            limit: None,
            stream_arn: None,
        })
        .unwrap();
    assert_eq!(records.records.len(), 1);
    assert_eq!(records.records[0].data, data);

    // Reset
    new_state.reset();
    let empty_list = new_state
        .list_streams(ListStreamsRequest::default())
        .unwrap();
    assert_eq!(empty_list.stream_names.len(), 0);
}

#[tokio::test]
async fn test_http_handlers() {
    let state = KinesisState::new("000000000000".to_string(), "us-east-1".to_string());

    // 1. Create Stream via HTTP handler
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-amz-target",
        "Kinesis_20131202.CreateStream".parse().unwrap(),
    );
    let body = Bytes::from(r#"{"StreamName":"http-stream","ShardCount":1}"#);

    let resp = handle_kinesis_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    // 2. Put Record via HTTP handler
    headers.insert(
        "x-amz-target",
        "Kinesis_20131202.PutRecord".parse().unwrap(),
    );
    let data = base64::engine::general_purpose::STANDARD.encode("Hello HTTP Kinesis");
    let put_body = Bytes::from(format!(
        r#"{{"StreamName":"http-stream","Data":"{}","PartitionKey":"key-1"}}"#,
        data
    ));

    let put_resp = handle_kinesis_request(State(state.clone()), headers.clone(), put_body).await;
    assert_eq!(put_resp.status(), axum::http::StatusCode::OK);

    // 3. Get Shard Iterator via HTTP handler
    headers.insert(
        "x-amz-target",
        "Kinesis_20131202.GetShardIterator".parse().unwrap(),
    );
    let iter_body = Bytes::from(
        r#"{"StreamName":"http-stream","ShardId":"shardId-000000000000","ShardIteratorType":"TRIM_HORIZON"}"#,
    );

    let iter_resp = handle_kinesis_request(State(state.clone()), headers.clone(), iter_body).await;
    assert_eq!(iter_resp.status(), axum::http::StatusCode::OK);

    let iter_bytes = axum::body::to_bytes(iter_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let iter_json: serde_json::Value = serde_json::from_slice(&iter_bytes).unwrap();
    let shard_iter = iter_json["ShardIterator"].as_str().unwrap();

    // 4. Get Records via HTTP handler
    headers.insert(
        "x-amz-target",
        "Kinesis_20131202.GetRecords".parse().unwrap(),
    );
    let get_body = Bytes::from(format!(r#"{{"ShardIterator":"{}"}}"#, shard_iter));

    let get_resp = handle_kinesis_request(State(state.clone()), headers.clone(), get_body).await;
    assert_eq!(get_resp.status(), axum::http::StatusCode::OK);

    let get_bytes = axum::body::to_bytes(get_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let get_json: serde_json::Value = serde_json::from_slice(&get_bytes).unwrap();
    assert_eq!(get_json["Records"].as_array().unwrap().len(), 1);
    assert_eq!(get_json["Records"][0]["Data"].as_str().unwrap(), data);
}

#[tokio::test]
async fn test_kinesis_resharding_split_merge_update_count() {
    let state = KinesisState::new("000000000000".to_string(), "us-east-1".to_string());

    // 1. Create Stream with 1 shard
    state
        .create_stream(CreateStreamRequest {
            stream_name: "reshard-stream".to_string(),
            shard_count: Some(1),
            stream_mode_details: None,
        })
        .unwrap();

    let desc = state
        .describe_stream(DescribeStreamRequest {
            stream_name: Some("reshard-stream".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(desc.shards.len(), 1);

    // 2. Split Shard
    let mid_hash = (u128::MAX / 2).to_string();
    state
        .split_shard(SplitShardRequest {
            stream_name: Some("reshard-stream".to_string()),
            stream_arn: None,
            shard_to_split: "shardId-000000000000".to_string(),
            new_starting_hash_key: mid_hash,
        })
        .unwrap();

    let desc_after_split = state
        .describe_stream(DescribeStreamRequest {
            stream_name: Some("reshard-stream".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(desc_after_split.shards.len(), 3); // 1 closed parent + 2 open children

    // 3. Merge the two children back
    state
        .merge_shards(MergeShardsRequest {
            stream_name: Some("reshard-stream".to_string()),
            stream_arn: None,
            shard_to_merge: "shardId-000000000001".to_string(),
            adjacent_shard_to_merge: "shardId-000000000002".to_string(),
        })
        .unwrap();

    let desc_after_merge = state
        .describe_stream(DescribeStreamRequest {
            stream_name: Some("reshard-stream".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(desc_after_merge.shards.len(), 4); // 3 closed + 1 open merged

    // 4. Update Shard Count to 4
    let update_res = state
        .update_shard_count(UpdateShardCountRequest {
            stream_name: Some("reshard-stream".to_string()),
            stream_arn: None,
            target_shard_count: 4,
            scaling_type: "UNIFORM_SCALING".to_string(),
        })
        .unwrap();
    assert_eq!(update_res.current_shard_count, 1);
    assert_eq!(update_res.target_shard_count, 4);

    let summary = state
        .describe_stream_summary(DescribeStreamSummaryRequest {
            stream_name: Some("reshard-stream".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(summary.open_shard_count, 4);
}
