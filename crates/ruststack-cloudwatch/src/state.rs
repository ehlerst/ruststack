use crate::types::*;
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum CloudWatchError {
    #[error("InvalidParameterValue: {0}")]
    InvalidParameter(String),

    #[error("ResourceNotFound: {0}")]
    ResourceNotFound(String),

    #[error("MissingParameter: {0}")]
    MissingParameter(String),

    #[error("InternalError: {0}")]
    Internal(String),
}

impl CloudWatchError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidParameter(_) | Self::MissingParameter(_) => StatusCode::BAD_REQUEST,
            Self::ResourceNotFound(_) => StatusCode::NOT_FOUND,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_code(&self) -> &str {
        match self {
            Self::InvalidParameter(_) => "InvalidParameterValue",
            Self::ResourceNotFound(_) => "ResourceNotFound",
            Self::MissingParameter(_) => "MissingParameter",
            Self::Internal(_) => "InternalError",
        }
    }

    pub fn to_xml(&self, request_id: &str) -> String {
        let code = self.error_code();
        let message = match self {
            Self::InvalidParameter(msg)
            | Self::ResourceNotFound(msg)
            | Self::MissingParameter(msg)
            | Self::Internal(msg) => msg.as_str(),
        };

        format!(
            r#"<ErrorResponse xmlns="http://monitoring.amazonaws.com/doc/2010-08-01/">
    <Error>
        <Type>Sender</Type>
        <Code>{}</Code>
        <Message>{}</Message>
    </Error>
    <RequestId>{}</RequestId>
</ErrorResponse>"#,
            quick_xml::escape::escape(code),
            quick_xml::escape::escape(message),
            quick_xml::escape::escape(request_id)
        )
    }

    pub fn to_json(&self) -> serde_json::Value {
        let code = self.error_code();
        let message = match self {
            Self::InvalidParameter(msg)
            | Self::ResourceNotFound(msg)
            | Self::MissingParameter(msg)
            | Self::Internal(msg) => msg.clone(),
        };

        serde_json::json!({
            "__type": code,
            "message": message
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetricKey {
    pub namespace: String,
    pub metric_name: String,
    pub dimensions: Vec<Dimension>,
}

impl MetricKey {
    pub fn new(namespace: String, metric_name: String, mut dimensions: Vec<Dimension>) -> Self {
        dimensions.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.value.cmp(&b.value)));
        Self {
            namespace,
            metric_name,
            dimensions,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSeries {
    pub key: MetricKey,
    pub datapoints: Vec<RawDataPoint>,
}

#[derive(Clone)]
pub struct CloudWatchState {
    pub account_id: String,
    pub region: String,
    pub series: Arc<DashMap<MetricKey, Arc<RwLock<MetricSeries>>>>,
    pub alarms: Arc<DashMap<String, MetricAlarm>>,
    pub sns_engine: Arc<RwLock<Option<Arc<ruststack_sns::SnsEngine>>>>,
}

impl CloudWatchState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            series: Arc::new(DashMap::new()),
            alarms: Arc::new(DashMap::new()),
            sns_engine: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_sns_engine(&self, engine: Arc<ruststack_sns::SnsEngine>) {
        *self.sns_engine.write() = Some(engine);
    }

    pub fn alarm_arn(&self, alarm_name: &str) -> String {
        format!(
            "arn:aws:cloudwatch:{}:{}:alarm:{}",
            self.region, self.account_id, alarm_name
        )
    }

    pub fn put_metric_data(&self, req: PutMetricDataRequest) -> Result<(), CloudWatchError> {
        if req.namespace.trim().is_empty() {
            return Err(CloudWatchError::MissingParameter(
                "Namespace is required".to_string(),
            ));
        }

        let now = Utc::now();

        for datum in req.metric_data {
            if datum.metric_name.trim().is_empty() {
                return Err(CloudWatchError::MissingParameter(
                    "MetricName is required in MetricDatum".to_string(),
                ));
            }

            let dimensions = datum.dimensions.unwrap_or_default();
            let key = MetricKey::new(req.namespace.clone(), datum.metric_name.clone(), dimensions);
            let ts = datum.timestamp.unwrap_or(now);

            let mut new_points = Vec::new();

            if let Some(stats) = datum.statistic_values {
                let val = if stats.sample_count > 0.0 {
                    stats.sum / stats.sample_count
                } else {
                    0.0
                };
                new_points.push(RawDataPoint {
                    timestamp: ts,
                    value: val,
                    unit: datum.unit.clone(),
                    sample_count: stats.sample_count,
                    sum: stats.sum,
                    min: stats.minimum,
                    max: stats.maximum,
                });
            } else if let Some(values) = datum.values {
                let counts = datum.counts.unwrap_or_default();
                for (i, &v) in values.iter().enumerate() {
                    let count = counts.get(i).copied().unwrap_or(1.0);
                    new_points.push(RawDataPoint {
                        timestamp: ts,
                        value: v,
                        unit: datum.unit.clone(),
                        sample_count: count,
                        sum: v * count,
                        min: v,
                        max: v,
                    });
                }
            } else if let Some(v) = datum.value {
                new_points.push(RawDataPoint {
                    timestamp: ts,
                    value: v,
                    unit: datum.unit.clone(),
                    sample_count: 1.0,
                    sum: v,
                    min: v,
                    max: v,
                });
            } else {
                new_points.push(RawDataPoint {
                    timestamp: ts,
                    value: 0.0,
                    unit: datum.unit.clone(),
                    sample_count: 1.0,
                    sum: 0.0,
                    min: 0.0,
                    max: 0.0,
                });
            }

            let entry = self.series.entry(key.clone()).or_insert_with(|| {
                Arc::new(RwLock::new(MetricSeries {
                    key,
                    datapoints: Vec::new(),
                }))
            });

            let mut series_lock = entry.value().write();
            series_lock.datapoints.extend(new_points);
        }

        Ok(())
    }

    pub fn list_metrics(
        &self,
        req: ListMetricsRequest,
    ) -> Result<ListMetricsResponse, CloudWatchError> {
        let mut metrics = Vec::new();

        for entry in self.series.iter() {
            let key = entry.key();

            if let Some(ref ns) = req.namespace {
                if &key.namespace != ns {
                    continue;
                }
            }

            if let Some(ref mn) = req.metric_name {
                if &key.metric_name != mn {
                    continue;
                }
            }

            if let Some(ref dim_filters) = req.dimensions {
                let mut matches_all = true;
                for filter in dim_filters {
                    let found = key.dimensions.iter().any(|d| {
                        if d.name != filter.name {
                            return false;
                        }
                        if let Some(ref val) = filter.value {
                            &d.value == val
                        } else {
                            true
                        }
                    });
                    if !found {
                        matches_all = false;
                        break;
                    }
                }
                if !matches_all {
                    continue;
                }
            }

            metrics.push(Metric {
                namespace: Some(key.namespace.clone()),
                metric_name: Some(key.metric_name.clone()),
                dimensions: if key.dimensions.is_empty() {
                    None
                } else {
                    Some(key.dimensions.clone())
                },
            });
        }

        metrics.sort_by(|a, b| {
            a.namespace
                .cmp(&b.namespace)
                .then_with(|| a.metric_name.cmp(&b.metric_name))
        });

        Ok(ListMetricsResponse {
            metrics,
            next_token: None,
        })
    }

    pub fn get_metric_data(
        &self,
        req: GetMetricDataRequest,
    ) -> Result<GetMetricDataResponse, CloudWatchError> {
        let mut metric_data_results = Vec::new();
        let scan_desc = req
            .scan_by
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("TimestampDescending"))
            .unwrap_or(false);

        for query in req.metric_data_queries {
            if query.return_data == Some(false) {
                continue;
            }

            let mut timestamps = Vec::new();
            let mut values = Vec::new();

            if let Some(ms) = query.metric_stat {
                let namespace = ms.metric.namespace.unwrap_or_default();
                let metric_name = ms.metric.metric_name.unwrap_or_default();
                let dimensions = ms.metric.dimensions.unwrap_or_default();
                let key = MetricKey::new(namespace, metric_name, dimensions);

                let period = if ms.period > 0 { ms.period as i64 } else { 60 };
                let stat = ms.stat.to_lowercase();

                if let Some(series_entry) = self.series.get(&key) {
                    let series_lock = series_entry.value().read();

                    // Group points into period buckets
                    let mut buckets: BTreeMap<i64, (f64, f64, f64, f64)> = BTreeMap::new(); // bucket_ts -> (sample_count, sum, min, max)

                    for dp in &series_lock.datapoints {
                        if dp.timestamp >= req.start_time && dp.timestamp < req.end_time {
                            let bucket_ts = (dp.timestamp.timestamp() / period) * period;
                            let entry = buckets.entry(bucket_ts).or_insert((
                                0.0,
                                0.0,
                                f64::INFINITY,
                                f64::NEG_INFINITY,
                            ));
                            entry.0 += dp.sample_count;
                            entry.1 += dp.sum;
                            if dp.min < entry.2 {
                                entry.2 = dp.min;
                            }
                            if dp.max > entry.3 {
                                entry.3 = dp.max;
                            }
                        }
                    }

                    let mut bucket_vec: Vec<(i64, f64)> = Vec::new();
                    for (ts, (count, sum, min, max)) in buckets {
                        let val = match stat.as_str() {
                            "samplecount" => count,
                            "sum" => sum,
                            "minimum" => {
                                if min.is_infinite() {
                                    0.0
                                } else {
                                    min
                                }
                            }
                            "maximum" => {
                                if max.is_infinite() {
                                    0.0
                                } else {
                                    max
                                }
                            }
                            _ => {
                                // Default is Average
                                if count > 0.0 {
                                    sum / count
                                } else {
                                    0.0
                                }
                            }
                        };
                        bucket_vec.push((ts, val));
                    }

                    if scan_desc {
                        bucket_vec.sort_by(|a, b| b.0.cmp(&a.0));
                    } else {
                        bucket_vec.sort_by(|a, b| a.0.cmp(&b.0));
                    }

                    for (ts, val) in bucket_vec {
                        if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                            timestamps.push(dt);
                            values.push(val);
                        }
                    }
                }
            }

            metric_data_results.push(MetricDataResult {
                id: query.id,
                label: query.label,
                timestamps,
                values,
                status_code: "Complete".to_string(),
                messages: Vec::new(),
            });
        }

        Ok(GetMetricDataResponse {
            metric_data_results,
            next_token: None,
            messages: Vec::new(),
        })
    }

    pub fn get_metric_statistics(
        &self,
        req: GetMetricStatisticsRequest,
    ) -> Result<GetMetricStatisticsResponse, CloudWatchError> {
        let dimensions = req.dimensions.unwrap_or_default();
        let key = MetricKey::new(req.namespace, req.metric_name.clone(), dimensions);
        let period = if req.period > 0 {
            req.period as i64
        } else {
            60
        };

        let mut datapoints = Vec::new();

        if let Some(series_entry) = self.series.get(&key) {
            let series_lock = series_entry.value().read();

            let mut buckets: BTreeMap<i64, (f64, f64, f64, f64, Option<String>)> = BTreeMap::new();

            for dp in &series_lock.datapoints {
                if dp.timestamp >= req.start_time && dp.timestamp < req.end_time {
                    let bucket_ts = (dp.timestamp.timestamp() / period) * period;
                    let entry = buckets.entry(bucket_ts).or_insert((
                        0.0,
                        0.0,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        dp.unit.clone(),
                    ));
                    entry.0 += dp.sample_count;
                    entry.1 += dp.sum;
                    if dp.min < entry.2 {
                        entry.2 = dp.min;
                    }
                    if dp.max > entry.3 {
                        entry.3 = dp.max;
                    }
                }
            }

            let requested_stats: Vec<String> = req
                .statistics
                .unwrap_or_default()
                .into_iter()
                .map(|s| s.to_lowercase())
                .collect();

            for (ts, (count, sum, min, max, unit)) in buckets {
                let dt = DateTime::from_timestamp(ts, 0);
                let mut dp = Datapoint {
                    timestamp: dt,
                    unit: req.unit.clone().or(unit),
                    ..Default::default()
                };

                if requested_stats.is_empty() {
                    dp.sample_count = Some(count);
                    dp.sum = Some(sum);
                    dp.average = Some(if count > 0.0 { sum / count } else { 0.0 });
                    dp.minimum = Some(if min.is_infinite() { 0.0 } else { min });
                    dp.maximum = Some(if max.is_infinite() { 0.0 } else { max });
                } else {
                    for stat in &requested_stats {
                        match stat.as_str() {
                            "samplecount" => dp.sample_count = Some(count),
                            "sum" => dp.sum = Some(sum),
                            "average" => {
                                dp.average = Some(if count > 0.0 { sum / count } else { 0.0 })
                            }
                            "minimum" => {
                                dp.minimum = Some(if min.is_infinite() { 0.0 } else { min })
                            }
                            "maximum" => {
                                dp.maximum = Some(if max.is_infinite() { 0.0 } else { max })
                            }
                            _ => {}
                        }
                    }
                }

                datapoints.push(dp);
            }
        }

        datapoints.sort_by_key(|dp| dp.timestamp);

        Ok(GetMetricStatisticsResponse {
            label: Some(req.metric_name),
            datapoints,
        })
    }

    pub fn put_metric_alarm(&self, req: PutMetricAlarmRequest) -> Result<(), CloudWatchError> {
        if req.alarm_name.trim().is_empty() {
            return Err(CloudWatchError::MissingParameter(
                "AlarmName is required".to_string(),
            ));
        }

        let now = Utc::now();
        let arn = self.alarm_arn(&req.alarm_name);

        let (state_val, state_reason, state_reason_data, state_updated_ts) =
            if let Some(existing) = self.alarms.get(&req.alarm_name) {
                (
                    existing.state_value.clone(),
                    existing.state_reason.clone(),
                    existing.state_reason_data.clone(),
                    existing.state_updated_timestamp,
                )
            } else {
                (
                    "INSUFFICIENT_DATA".to_string(),
                    Some("Unchecked: Initial alarm creation".to_string()),
                    None,
                    Some(now),
                )
            };

        let alarm = MetricAlarm {
            alarm_name: req.alarm_name.clone(),
            alarm_arn: arn,
            alarm_description: req.alarm_description,
            alarm_configuration_updated_timestamp: Some(now),
            actions_enabled: req.actions_enabled.or(Some(true)),
            ok_actions: req.ok_actions,
            alarm_actions: req.alarm_actions,
            insufficient_data_actions: req.insufficient_data_actions,
            state_value: state_val,
            state_reason,
            state_reason_data,
            state_updated_timestamp: state_updated_ts,
            metric_name: req.metric_name,
            namespace: req.namespace,
            statistic: req.statistic,
            extended_statistic: req.extended_statistic,
            dimensions: req.dimensions,
            period: req.period,
            unit: req.unit,
            evaluation_periods: Some(req.evaluation_periods),
            datapoints_to_alarm: req.datapoints_to_alarm,
            threshold: req.threshold,
            comparison_operator: Some(req.comparison_operator),
            treat_missing_data: req.treat_missing_data,
            evaluate_low_sample_count_percentile: req.evaluate_low_sample_count_percentile,
            metrics: req.metrics,
            threshold_metric_id: req.threshold_metric_id,
        };

        self.alarms.insert(req.alarm_name, alarm);
        Ok(())
    }

    pub fn describe_alarms(
        &self,
        req: DescribeAlarmsRequest,
    ) -> Result<DescribeAlarmsResponse, CloudWatchError> {
        let mut metric_alarms = Vec::new();

        for item in self.alarms.iter() {
            let alarm = item.value();

            if let Some(ref names) = req.alarm_names {
                if !names.is_empty() && !names.contains(&alarm.alarm_name) {
                    continue;
                }
            }

            if let Some(ref prefix) = req.alarm_name_prefix {
                if !alarm.alarm_name.starts_with(prefix) {
                    continue;
                }
            }

            if let Some(ref state) = req.state_value {
                if !alarm.state_value.eq_ignore_ascii_case(state) {
                    continue;
                }
            }

            if let Some(ref action_prefix) = req.action_prefix {
                let mut matched = false;
                if let Some(ref actions) = alarm.alarm_actions {
                    if actions.iter().any(|a| a.starts_with(action_prefix)) {
                        matched = true;
                    }
                }
                if let Some(ref actions) = alarm.ok_actions {
                    if actions.iter().any(|a| a.starts_with(action_prefix)) {
                        matched = true;
                    }
                }
                if let Some(ref actions) = alarm.insufficient_data_actions {
                    if actions.iter().any(|a| a.starts_with(action_prefix)) {
                        matched = true;
                    }
                }
                if !matched {
                    continue;
                }
            }

            metric_alarms.push(alarm.clone());
        }

        metric_alarms.sort_by(|a, b| a.alarm_name.cmp(&b.alarm_name));

        if let Some(max_records) = req.max_records {
            if max_records > 0 && metric_alarms.len() > max_records as usize {
                metric_alarms.truncate(max_records as usize);
            }
        }

        Ok(DescribeAlarmsResponse {
            metric_alarms,
            composite_alarms: Vec::new(),
            next_token: None,
        })
    }

    pub fn describe_alarms_for_metric(
        &self,
        metric_name: &str,
        namespace: &str,
        dimensions: Option<&[Dimension]>,
    ) -> Result<Vec<MetricAlarm>, CloudWatchError> {
        let mut results = Vec::new();

        for item in self.alarms.iter() {
            let alarm = item.value();
            if alarm.metric_name.as_deref() != Some(metric_name) {
                continue;
            }
            if alarm.namespace.as_deref() != Some(namespace) {
                continue;
            }
            if let Some(dims) = dimensions {
                let alarm_dims = alarm.dimensions.as_deref().unwrap_or_default();
                if alarm_dims != dims {
                    continue;
                }
            }
            results.push(alarm.clone());
        }

        results.sort_by(|a, b| a.alarm_name.cmp(&b.alarm_name));
        Ok(results)
    }

    pub fn delete_alarms(&self, req: DeleteAlarmsRequest) -> Result<(), CloudWatchError> {
        for name in req.alarm_names {
            self.alarms.remove(&name);
        }
        Ok(())
    }

    pub fn set_alarm_state(&self, req: SetAlarmStateRequest) -> Result<(), CloudWatchError> {
        let (alarm_clone, old_state) = if let Some(mut entry) = self.alarms.get_mut(&req.alarm_name) {
            let old_state = entry.state_value.clone();
            entry.state_value = req.state_value.clone();
            entry.state_reason = Some(req.state_reason.clone());
            entry.state_reason_data = req.state_reason_data.clone();
            entry.state_updated_timestamp = Some(Utc::now());
            (entry.clone(), old_state)
        } else {
            return Err(CloudWatchError::ResourceNotFound(format!(
                "Alarm {} not found",
                req.alarm_name
            )));
        };

        // Dispatch Alarm Actions if enabled
        if alarm_clone.actions_enabled.unwrap_or(true) {
            let actions = match req.state_value.to_uppercase().as_str() {
                "ALARM" => alarm_clone.alarm_actions.clone().unwrap_or_default(),
                "OK" => alarm_clone.ok_actions.clone().unwrap_or_default(),
                "INSUFFICIENT_DATA" => {
                    alarm_clone.insufficient_data_actions.clone().unwrap_or_default()
                }
                _ => Vec::new(),
            };

            let sns_opt = self.sns_engine.read().clone();
            if let Some(sns) = sns_opt {
                let msg_payload = serde_json::json!({
                    "AlarmName": req.alarm_name,
                    "AlarmDescription": alarm_clone.alarm_description,
                    "AWSAccountId": self.account_id,
                    "NewStateValue": req.state_value,
                    "NewStateReason": req.state_reason,
                    "StateChangeTime": Utc::now().to_rfc3339(),
                    "Region": self.region,
                    "OldStateValue": old_state
                })
                .to_string();

                let subject = format!("ALARM: \"{}\" in {}", req.alarm_name, self.region);

                for action_arn in actions {
                    if action_arn.starts_with("arn:aws:sns:") {
                        let _ = sns.publish(
                            &action_arn,
                            msg_payload.clone(),
                            Some(subject.clone()),
                            None,
                            None,
                            None,
                        );
                    }
                }
            }
        }

        Ok(())
    }

    pub fn enable_alarm_actions(&self, alarm_names: &[String]) -> Result<(), CloudWatchError> {
        for name in alarm_names {
            if let Some(mut entry) = self.alarms.get_mut(name) {
                entry.actions_enabled = Some(true);
            }
        }
        Ok(())
    }

    pub fn disable_alarm_actions(&self, alarm_names: &[String]) -> Result<(), CloudWatchError> {
        for name in alarm_names {
            if let Some(mut entry) = self.alarms.get_mut(name) {
                entry.actions_enabled = Some(false);
            }
        }
        Ok(())
    }

    pub fn export_snapshot(&self) -> CloudWatchSnapshot {
        let mut stored_metrics = Vec::new();
        for entry in self.series.iter() {
            let key = entry.key();
            let series_lock = entry.value().read();
            stored_metrics.push(StoredMetricSeries {
                namespace: key.namespace.clone(),
                metric_name: key.metric_name.clone(),
                dimensions: key.dimensions.clone(),
                datapoints: series_lock.datapoints.clone(),
            });
        }

        let mut alarms = Vec::new();
        for item in self.alarms.iter() {
            alarms.push(item.value().clone());
        }

        CloudWatchSnapshot {
            account_id: self.account_id.clone(),
            region: self.region.clone(),
            metrics: stored_metrics,
            alarms,
        }
    }

    pub fn import_snapshot(&self, snapshot: CloudWatchSnapshot) {
        self.series.clear();
        self.alarms.clear();

        for m in snapshot.metrics {
            let key = MetricKey::new(m.namespace, m.metric_name, m.dimensions);
            self.series.insert(
                key.clone(),
                Arc::new(RwLock::new(MetricSeries {
                    key,
                    datapoints: m.datapoints,
                })),
            );
        }

        for a in snapshot.alarms {
            self.alarms.insert(a.alarm_name.clone(), a);
        }
    }

    pub fn reset(&self) {
        self.series.clear();
        self.alarms.clear();
    }
}
