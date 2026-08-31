use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Dimension {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct DimensionFilter {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct StatisticSet {
    pub sample_count: f64,
    pub sum: f64,
    pub minimum: f64,
    pub maximum: f64,
}

pub type StatisticValues = StatisticSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MetricDatum {
    pub metric_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Vec<Dimension>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counts: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistic_values: Option<StatisticSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_resolution: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PutMetricDataRequest {
    pub namespace: String,
    pub metric_data: Vec<MetricDatum>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Metric {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Vec<Dimension>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListMetricsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Vec<DimensionFilter>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recently_active: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ListMetricsResponse {
    pub metrics: Vec<Metric>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MetricStat {
    pub metric: Metric,
    pub period: i32,
    pub stat: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MetricDataQuery {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric_stat: Option<MetricStat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_data: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetMetricDataRequest {
    pub metric_data_queries: Vec<MetricDataQuery>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_datapoints: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct MessageData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MetricDataResult {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub timestamps: Vec<DateTime<Utc>>,
    pub values: Vec<f64>,
    pub status_code: String,
    #[serde(default)]
    pub messages: Vec<MessageData>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct GetMetricDataResponse {
    pub metric_data_results: Vec<MetricDataResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
    #[serde(default)]
    pub messages: Vec<MessageData>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetMetricStatisticsRequest {
    pub namespace: String,
    pub metric_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Vec<Dimension>>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub period: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistics: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extended_statistics: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Datapoint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_count: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub average: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extended_statistics: Option<HashMap<String, f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct GetMetricStatisticsResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub datapoints: Vec<Datapoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MetricAlarm {
    pub alarm_name: String,
    pub alarm_arn: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alarm_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alarm_configuration_updated_timestamp: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok_actions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alarm_actions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insufficient_data_actions: Option<Vec<String>>,
    pub state_value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_reason_data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_updated_timestamp: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extended_statistic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Vec<Dimension>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_periods: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datapoints_to_alarm: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparison_operator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treat_missing_data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluate_low_sample_count_percentile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Vec<MetricDataQuery>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_metric_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PutMetricAlarmRequest {
    pub alarm_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alarm_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok_actions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alarm_actions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insufficient_data_actions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extended_statistic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Vec<Dimension>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub evaluation_periods: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datapoints_to_alarm: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    pub comparison_operator: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treat_missing_data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluate_low_sample_count_percentile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Vec<MetricDataQuery>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_metric_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeAlarmsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alarm_names: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alarm_name_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alarm_types: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children_of_alarm_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parents_of_alarm_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_records: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeAlarmsResponse {
    pub metric_alarms: Vec<MetricAlarm>,
    #[serde(default)]
    pub composite_alarms: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteAlarmsRequest {
    pub alarm_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SetAlarmStateRequest {
    pub alarm_name: String,
    pub state_value: String,
    pub state_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_reason_data: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawDataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub unit: Option<String>,
    pub sample_count: f64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredMetricSeries {
    pub namespace: String,
    pub metric_name: String,
    pub dimensions: Vec<Dimension>,
    pub datapoints: Vec<RawDataPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloudWatchSnapshot {
    pub account_id: String,
    pub region: String,
    pub metrics: Vec<StoredMetricSeries>,
    pub alarms: Vec<MetricAlarm>,
}
