use axum::http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;

#[tokio::test]
async fn test_cloudwatch_metrics_and_alarms_compat() {
    let client = RustStackTestClient::new();

    // 1. Put Metric Data
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "PutMetricData"),
                ("Namespace", "AWS/EC2"),
                ("MetricData.member.1.MetricName", "CPUUtilization"),
                ("MetricData.member.1.Value", "45.2"),
                ("MetricData.member.1.Unit", "Percent"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("PutMetricDataResponse"));

    // 2. List Metrics
    let (status, body) = client
        .call_query("/", &[("Action", "ListMetrics"), ("Namespace", "AWS/EC2")])
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("CPUUtilization"));

    // 3. Put Metric Alarm
    let (status, body) = client
        .call_query(
            "/",
            &[
                ("Action", "PutMetricAlarm"),
                ("AlarmName", "HighCPUAlarm"),
                ("MetricName", "CPUUtilization"),
                ("Namespace", "AWS/EC2"),
                ("Threshold", "80"),
                ("ComparisonOperator", "GreaterThanThreshold"),
                ("EvaluationPeriods", "1"),
                ("Period", "300"),
                ("Statistic", "Average"),
            ],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("PutMetricAlarmResponse"));

    // 4. Describe Alarms
    let (status, body) = client
        .call_query(
            "/",
            &[("Action", "DescribeAlarms"), ("AlarmNamePrefix", "HighCPU")],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("HighCPUAlarm"));
}
