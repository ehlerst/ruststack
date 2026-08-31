use ruststack_logs::state::LogsState;
use ruststack_logs::types::*;

#[tokio::test]
async fn test_logs_lifecycle() {
    let state = LogsState::new("000000000000".to_string(), "us-east-1".to_string());

    // 1. Create log group
    state
        .create_log_group(CreateLogGroupRequest {
            log_group_name: "/aws/lambda/my-function".to_string(),
            retention_in_days: Some(7),
            tags: None,
        })
        .expect("create log group");

    // 2. Describe log groups
    let (groups, _) = state
        .describe_log_groups(DescribeLogGroupsRequest::default())
        .expect("describe log groups");
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].log_group_name, "/aws/lambda/my-function");
    assert_eq!(groups[0].retention_in_days, Some(7));

    // 3. Create log stream
    state
        .create_log_stream(CreateLogStreamRequest {
            log_group_name: "/aws/lambda/my-function".to_string(),
            log_stream_name: "2026/08/30/[$LATEST]abc123".to_string(),
        })
        .expect("create log stream");

    // 4. Describe log streams
    let (streams, _) = state
        .describe_log_streams(DescribeLogStreamsRequest {
            log_group_name: "/aws/lambda/my-function".to_string(),
            log_stream_name_prefix: None,
            order_by: None,
            descending: None,
            limit: None,
            next_token: None,
        })
        .expect("describe log streams");
    assert_eq!(streams.len(), 1);
    assert_eq!(streams[0].log_stream_name, "2026/08/30/[$LATEST]abc123");

    // 5. Put log events
    let seq = state
        .put_log_events(PutLogEventsRequest {
            log_group_name: "/aws/lambda/my-function".to_string(),
            log_stream_name: "2026/08/30/[$LATEST]abc123".to_string(),
            log_events: vec![
                InputLogEvent {
                    timestamp: 1000,
                    message: "START RequestId: 1234-abcd".to_string(),
                },
                InputLogEvent {
                    timestamp: 1050,
                    message: "INFO Processing invoice #9999".to_string(),
                },
                InputLogEvent {
                    timestamp: 1100,
                    message: "END RequestId: 1234-abcd".to_string(),
                },
            ],
            sequence_token: None,
        })
        .expect("put log events");
    assert_eq!(seq, "2");

    // 6. Get log events
    let events = state
        .get_log_events(GetLogEventsRequest {
            log_group_name: "/aws/lambda/my-function".to_string(),
            log_stream_name: "2026/08/30/[$LATEST]abc123".to_string(),
            start_time: Some(1000),
            end_time: Some(1200),
            limit: Some(10),
            next_token: None,
            start_from_head: Some(true),
        })
        .expect("get log events");
    assert_eq!(events.len(), 3);
    assert_eq!(events[1].message, "INFO Processing invoice #9999");

    // 7. Filter log events
    let filtered = state
        .filter_log_events(FilterLogEventsRequest {
            log_group_name: "/aws/lambda/my-function".to_string(),
            log_stream_names: None,
            log_stream_name_prefix: None,
            start_time: None,
            end_time: None,
            filter_pattern: Some("invoice".to_string()),
            limit: None,
            next_token: None,
        })
        .expect("filter log events");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].message, "INFO Processing invoice #9999");

    // 8. Snapshot export and reload
    let snap = state.export_snapshot();
    let new_state = LogsState::new("000000000000".to_string(), "us-east-1".to_string());
    new_state.import_snapshot(snap);

    let (groups_after, _) = new_state
        .describe_log_groups(DescribeLogGroupsRequest::default())
        .expect("describe log groups after reload");
    assert_eq!(groups_after.len(), 1);

    // 9. Reset
    new_state.reset();
    let (empty_groups, _) = new_state
        .describe_log_groups(DescribeLogGroupsRequest::default())
        .expect("describe empty log groups");
    assert_eq!(empty_groups.len(), 0);
}
