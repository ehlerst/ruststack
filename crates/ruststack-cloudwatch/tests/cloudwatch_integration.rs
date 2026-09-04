use ruststack_cloudwatch::types::*;
use ruststack_cloudwatch::CloudWatchState;

#[test]
fn test_cloudwatch_metric_and_alarm_lifecycle() {
    let state = CloudWatchState::new("000000000000".to_string(), "us-east-1".to_string());

    // 1. Put Metric Data
    let put_req = PutMetricDataRequest {
        namespace: "AWS/Billing".to_string(),
        metric_data: vec![MetricDatum {
            metric_name: "EstimatedCharges".to_string(),
            dimensions: Some(vec![Dimension {
                name: "Currency".to_string(),
                value: "USD".to_string(),
            }]),
            timestamp: None,
            value: Some(42.50),
            values: None,
            counts: None,
            unit: Some("None".to_string()),
            statistic_values: None,
            storage_resolution: None,
        }],
    };
    state.put_metric_data(put_req).unwrap();

    // 2. List Metrics
    let list_req = ListMetricsRequest {
        namespace: Some("AWS/Billing".to_string()),
        metric_name: None,
        dimensions: None,
        next_token: None,
        recently_active: None,
    };
    let list_resp = state.list_metrics(list_req).unwrap();
    assert_eq!(list_resp.metrics.len(), 1);
    assert_eq!(
        list_resp.metrics[0].metric_name.as_deref(),
        Some("EstimatedCharges")
    );

    // 3. Put Alarm
    let alarm_req = PutMetricAlarmRequest {
        alarm_name: "HighCharges".to_string(),
        alarm_description: Some("Notify if charge > 50".to_string()),
        actions_enabled: Some(true),
        ok_actions: None,
        alarm_actions: Some(vec!["arn:aws:sns:us-east-1:000000000000:alerts".to_string()]),
        insufficient_data_actions: None,
        metric_name: Some("EstimatedCharges".to_string()),
        namespace: Some("AWS/Billing".to_string()),
        statistic: Some("Maximum".to_string()),
        extended_statistic: None,
        dimensions: None,
        period: Some(300),
        unit: None,
        evaluation_periods: 1,
        datapoints_to_alarm: None,
        threshold: Some(50.0),
        comparison_operator: "GreaterThanThreshold".to_string(),
        treat_missing_data: None,
        evaluate_low_sample_count_percentile: None,
        metrics: None,
        tags: None,
        threshold_metric_id: None,
    };
    state.put_metric_alarm(alarm_req).unwrap();

    // 4. Describe Alarms
    let desc_req = DescribeAlarmsRequest {
        alarm_names: Some(vec!["HighCharges".to_string()]),
        alarm_name_prefix: None,
        alarm_types: None,
        children_of_alarm_name: None,
        parents_of_alarm_name: None,
        state_value: None,
        action_prefix: None,
        max_records: None,
        next_token: None,
    };
    let desc_resp = state.describe_alarms(desc_req).unwrap();
    assert_eq!(desc_resp.metric_alarms.len(), 1);
    assert_eq!(desc_resp.metric_alarms[0].alarm_name, "HighCharges");

    // 5. Delete Alarm
    let del_req = DeleteAlarmsRequest {
        alarm_names: vec!["HighCharges".to_string()],
    };
    state.delete_alarms(del_req).unwrap();
    let desc_resp_after = state
        .describe_alarms(DescribeAlarmsRequest {
            alarm_names: Some(vec!["HighCharges".to_string()]),
            alarm_name_prefix: None,
            alarm_types: None,
            children_of_alarm_name: None,
            parents_of_alarm_name: None,
            state_value: None,
            action_prefix: None,
            max_records: None,
            next_token: None,
        })
        .unwrap();
    assert_eq!(desc_resp_after.metric_alarms.len(), 0);
}

#[tokio::test]
async fn test_cloudwatch_alarm_action_sns_notification() {
    let cw_state = std::sync::Arc::new(CloudWatchState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let sqs = std::sync::Arc::new(ruststack_sqs::SqsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let sns = std::sync::Arc::new(ruststack_sns::SnsEngine::new(
        sqs.clone(),
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    cw_state.set_sns_engine(sns.clone());

    // 1. Create SQS Queue and SNS Topic, and subscribe
    let queue_url = sqs.create_queue("alarm-notifications", None).unwrap();
    let topic_arn = sns.create_topic("ops-alerts", None).unwrap();
    sns.subscribe(&topic_arn, "sqs", &queue_url, None).unwrap();

    // 2. Put Metric Alarm with AlarmActions pointing to SNS Topic
    let alarm_req = PutMetricAlarmRequest {
        alarm_name: "CpuUtilizationHigh".to_string(),
        alarm_description: Some("CPU > 90%".to_string()),
        actions_enabled: Some(true),
        ok_actions: None,
        alarm_actions: Some(vec![topic_arn.clone()]),
        insufficient_data_actions: None,
        metric_name: Some("CPUUtilization".to_string()),
        namespace: Some("AWS/EC2".to_string()),
        statistic: Some("Average".to_string()),
        extended_statistic: None,
        dimensions: None,
        period: Some(60),
        unit: None,
        evaluation_periods: 1,
        datapoints_to_alarm: None,
        threshold: Some(90.0),
        comparison_operator: "GreaterThanThreshold".to_string(),
        treat_missing_data: None,
        evaluate_low_sample_count_percentile: None,
        metrics: None,
        tags: None,
        threshold_metric_id: None,
    };
    cw_state.put_metric_alarm(alarm_req).unwrap();

    // 3. Set Alarm State to ALARM
    let set_state_req = SetAlarmStateRequest {
        alarm_name: "CpuUtilizationHigh".to_string(),
        state_value: "ALARM".to_string(),
        state_reason: "Threshold Crossed: 1 out of 1 datapoints [95.0] > threshold [90.0]".to_string(),
        state_reason_data: None,
    };
    cw_state.set_alarm_state(set_state_req).unwrap();

    // 4. Verify message received in SQS queue
    let msgs = sqs.receive_message(&queue_url, 10, None, None).await.unwrap();
    assert_eq!(msgs.len(), 1);
    let body_str = &msgs[0].body;
    assert!(body_str.contains("CpuUtilizationHigh"));
    assert!(body_str.contains("ALARM"));
}
