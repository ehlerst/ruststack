use crate::state::{CloudWatchError, CloudWatchState};
use crate::types::*;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use quick_xml::escape::escape;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn handle_cloudwatch_request(
    State(state): State<CloudWatchState>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = Uuid::new_v4().to_string();

    let is_json = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .map(|t| t.starts_with("GraniteServiceVersion20100801") || t.starts_with("CloudWatch"))
        .unwrap_or(false)
        || headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|ct| ct.contains("application/x-amz-json") || ct.contains("application/json"))
            .unwrap_or(false);

    let mut params: HashMap<String, String> = HashMap::new();

    // 1. Query parameters
    if let Some(query) = uri.query() {
        for (k, v) in form_urlencoded::parse(query.as_bytes()) {
            params.insert(k.into_owned(), v.into_owned());
        }
    }

    // 2. Form url-encoded or JSON body
    if !body.is_empty() {
        if let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(action) = json_val.get("Action").and_then(|v| v.as_str()) {
                params.insert("Action".to_string(), action.to_string());
            }
        }
        for (k, v) in form_urlencoded::parse(&body) {
            params.insert(k.into_owned(), v.into_owned());
        }
    }

    // Check target header for action
    if let Some(target) = headers.get("x-amz-target").and_then(|v| v.to_str().ok()) {
        let action_name = if let Some(pos) = target.find('.') {
            &target[pos + 1..]
        } else {
            target
        };
        params.insert("Action".to_string(), action_name.to_string());
    }

    let action = match params.get("Action") {
        Some(a) => a.clone(),
        None => {
            return if is_json {
                (
                    StatusCode::BAD_REQUEST,
                    [("content-type", "application/x-amz-json-1.1")],
                    json!({ "__type": "MissingAction", "message": "Missing Action" }).to_string(),
                )
                    .into_response()
            } else {
                (
                    StatusCode::BAD_REQUEST,
                    [("content-type", "text/xml")],
                    format!(
                        r#"<ErrorResponse xmlns="http://monitoring.amazonaws.com/doc/2010-08-01/"><Error><Type>Sender</Type><Code>MissingAction</Code><Message>Missing Action</Message></Error><RequestId>{}</RequestId></ErrorResponse>"#,
                        request_id
                    ),
                )
                    .into_response()
            };
        }
    };

    match action.as_str() {
        "PutMetricData" => {
            let req = parse_put_metric_data_request(&params, &body);
            match state.put_metric_data(req) {
                Ok(()) => {
                    if is_json {
                        (
                            StatusCode::OK,
                            [("content-type", "application/x-amz-json-1.1")],
                            json!({}).to_string(),
                        )
                            .into_response()
                    } else {
                        let xml = format!(
                            r#"<PutMetricDataResponse xmlns="http://monitoring.amazonaws.com/doc/2010-08-01/"><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></PutMetricDataResponse>"#,
                            request_id
                        );
                        (StatusCode::OK, [("content-type", "text/xml")], xml).into_response()
                    }
                }
                Err(e) => format_error_response(e, is_json, &request_id),
            }
        }

        "ListMetrics" => {
            let req = parse_list_metrics_request(&params, &body);
            match state.list_metrics(req) {
                Ok(resp) => {
                    if is_json {
                        (
                            StatusCode::OK,
                            [("content-type", "application/x-amz-json-1.1")],
                            serde_json::to_string(&resp).unwrap_or_default(),
                        )
                            .into_response()
                    } else {
                        let mut metrics_xml = String::new();
                        for m in resp.metrics {
                            metrics_xml.push_str("<member>");
                            metrics_xml.push_str(&format!(
                                "<MetricName>{}</MetricName>",
                                escape(m.metric_name.as_deref().unwrap_or_default())
                            ));
                            metrics_xml.push_str(&format!(
                                "<Namespace>{}</Namespace>",
                                escape(m.namespace.as_deref().unwrap_or_default())
                            ));
                            if let Some(dimensions) = m.dimensions {
                                metrics_xml.push_str("<Dimensions>");
                                for d in dimensions {
                                    metrics_xml.push_str("<member>");
                                    metrics_xml
                                        .push_str(&format!("<Name>{}</Name>", escape(&d.name)));
                                    metrics_xml
                                        .push_str(&format!("<Value>{}</Value>", escape(&d.value)));
                                    metrics_xml.push_str("</member>");
                                }
                                metrics_xml.push_str("</Dimensions>");
                            }
                            metrics_xml.push_str("</member>");
                        }
                        let next_token_xml = resp
                            .next_token
                            .map(|t| format!("<NextToken>{}</NextToken>", escape(&t)))
                            .unwrap_or_default();

                        let xml = format!(
                            r#"<ListMetricsResponse xmlns="http://monitoring.amazonaws.com/doc/2010-08-01/"><ListMetricsResult><Metrics>{}</Metrics>{}</ListMetricsResult><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></ListMetricsResponse>"#,
                            metrics_xml, next_token_xml, request_id
                        );
                        (StatusCode::OK, [("content-type", "text/xml")], xml).into_response()
                    }
                }
                Err(e) => format_error_response(e, is_json, &request_id),
            }
        }

        "GetMetricData" => {
            let req = parse_get_metric_data_request(&params, &body);
            match state.get_metric_data(req) {
                Ok(resp) => {
                    if is_json {
                        (
                            StatusCode::OK,
                            [("content-type", "application/x-amz-json-1.1")],
                            serde_json::to_string(&resp).unwrap_or_default(),
                        )
                            .into_response()
                    } else {
                        let mut results_xml = String::new();
                        for r in resp.metric_data_results {
                            results_xml.push_str("<member>");
                            results_xml.push_str(&format!("<Id>{}</Id>", escape(&r.id)));
                            if let Some(lbl) = r.label {
                                results_xml.push_str(&format!("<Label>{}</Label>", escape(&lbl)));
                            }
                            results_xml.push_str("<StatusCode>Complete</StatusCode>");
                            results_xml.push_str("<Timestamps>");
                            for ts in r.timestamps {
                                results_xml
                                    .push_str(&format!("<member>{}</member>", ts.to_rfc3339()));
                            }
                            results_xml.push_str("</Timestamps>");
                            results_xml.push_str("<Values>");
                            for v in r.values {
                                results_xml.push_str(&format!("<member>{}</member>", v));
                            }
                            results_xml.push_str("</Values>");
                            results_xml.push_str("</member>");
                        }

                        let xml = format!(
                            r#"<GetMetricDataResponse xmlns="http://monitoring.amazonaws.com/doc/2010-08-01/"><GetMetricDataResult><MetricDataResults>{}</MetricDataResults></GetMetricDataResult><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></GetMetricDataResponse>"#,
                            results_xml, request_id
                        );
                        (StatusCode::OK, [("content-type", "text/xml")], xml).into_response()
                    }
                }
                Err(e) => format_error_response(e, is_json, &request_id),
            }
        }

        "GetMetricStatistics" => {
            let req = parse_get_metric_statistics_request(&params, &body);
            match state.get_metric_statistics(req) {
                Ok(resp) => {
                    if is_json {
                        (
                            StatusCode::OK,
                            [("content-type", "application/x-amz-json-1.1")],
                            serde_json::to_string(&resp).unwrap_or_default(),
                        )
                            .into_response()
                    } else {
                        let mut points_xml = String::new();
                        for p in resp.datapoints {
                            points_xml.push_str("<member>");
                            let ts_str = p.timestamp.map(|t| t.to_rfc3339()).unwrap_or_default();
                            points_xml.push_str(&format!("<Timestamp>{}</Timestamp>", ts_str));
                            if let Some(v) = p.average {
                                points_xml.push_str(&format!("<Average>{}</Average>", v));
                            }
                            if let Some(v) = p.sum {
                                points_xml.push_str(&format!("<Sum>{}</Sum>", v));
                            }
                            if let Some(v) = p.sample_count {
                                points_xml.push_str(&format!("<SampleCount>{}</SampleCount>", v));
                            }
                            if let Some(v) = p.minimum {
                                points_xml.push_str(&format!("<Minimum>{}</Minimum>", v));
                            }
                            if let Some(v) = p.maximum {
                                points_xml.push_str(&format!("<Maximum>{}</Maximum>", v));
                            }
                            points_xml.push_str("</member>");
                        }

                        let xml = format!(
                            r#"<GetMetricStatisticsResponse xmlns="http://monitoring.amazonaws.com/doc/2010-08-01/"><GetMetricStatisticsResult><Label>{}</Label><Datapoints>{}</Datapoints></GetMetricStatisticsResult><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></GetMetricStatisticsResponse>"#,
                            escape(&resp.label.unwrap_or_default()),
                            points_xml,
                            request_id
                        );
                        (StatusCode::OK, [("content-type", "text/xml")], xml).into_response()
                    }
                }
                Err(e) => format_error_response(e, is_json, &request_id),
            }
        }

        "PutMetricAlarm" => {
            let req = parse_put_metric_alarm_request(&params, &body);
            match state.put_metric_alarm(req) {
                Ok(()) => {
                    if is_json {
                        (
                            StatusCode::OK,
                            [("content-type", "application/x-amz-json-1.1")],
                            json!({}).to_string(),
                        )
                            .into_response()
                    } else {
                        let xml = format!(
                            r#"<PutMetricAlarmResponse xmlns="http://monitoring.amazonaws.com/doc/2010-08-01/"><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></PutMetricAlarmResponse>"#,
                            request_id
                        );
                        (StatusCode::OK, [("content-type", "text/xml")], xml).into_response()
                    }
                }
                Err(e) => format_error_response(e, is_json, &request_id),
            }
        }

        "DescribeAlarms" => {
            let req = parse_describe_alarms_request(&params, &body);
            match state.describe_alarms(req) {
                Ok(resp) => {
                    if is_json {
                        (
                            StatusCode::OK,
                            [("content-type", "application/x-amz-json-1.1")],
                            serde_json::to_string(&resp).unwrap_or_default(),
                        )
                            .into_response()
                    } else {
                        let mut alarms_xml = String::new();
                        for a in resp.metric_alarms {
                            alarms_xml.push_str("<member>");
                            alarms_xml.push_str(&format!(
                                "<AlarmName>{}</AlarmName>",
                                escape(&a.alarm_name)
                            ));
                            alarms_xml.push_str(&format!(
                                "<AlarmArn>{}</AlarmArn>",
                                escape(&a.alarm_arn)
                            ));
                            alarms_xml.push_str(&format!(
                                "<StateValue>{}</StateValue>",
                                escape(&a.state_value)
                            ));
                            if let Some(m) = a.metric_name {
                                alarms_xml
                                    .push_str(&format!("<MetricName>{}</MetricName>", escape(&m)));
                            }
                            if let Some(ns) = a.namespace {
                                alarms_xml
                                    .push_str(&format!("<Namespace>{}</Namespace>", escape(&ns)));
                            }
                            alarms_xml.push_str("</member>");
                        }

                        let xml = format!(
                            r#"<DescribeAlarmsResponse xmlns="http://monitoring.amazonaws.com/doc/2010-08-01/"><DescribeAlarmsResult><MetricAlarms>{}</MetricAlarms></DescribeAlarmsResult><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></DescribeAlarmsResponse>"#,
                            alarms_xml, request_id
                        );
                        (StatusCode::OK, [("content-type", "text/xml")], xml).into_response()
                    }
                }
                Err(e) => format_error_response(e, is_json, &request_id),
            }
        }

        "DeleteAlarms" => {
            let req = parse_delete_alarms_request(&params, &body);
            match state.delete_alarms(req) {
                Ok(()) => {
                    if is_json {
                        (
                            StatusCode::OK,
                            [("content-type", "application/x-amz-json-1.1")],
                            json!({}).to_string(),
                        )
                            .into_response()
                    } else {
                        let xml = format!(
                            r#"<DeleteAlarmsResponse xmlns="http://monitoring.amazonaws.com/doc/2010-08-01/"><ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata></DeleteAlarmsResponse>"#,
                            request_id
                        );
                        (StatusCode::OK, [("content-type", "text/xml")], xml).into_response()
                    }
                }
                Err(e) => format_error_response(e, is_json, &request_id),
            }
        }

        _ => {
            let err = CloudWatchError::InvalidParameter(format!("Unknown Action: {}", action));
            format_error_response(err, is_json, &request_id)
        }
    }
}

fn format_error_response(err: CloudWatchError, is_json: bool, request_id: &str) -> Response {
    if is_json {
        (
            err.status_code(),
            [("content-type", "application/x-amz-json-1.1")],
            err.to_json().to_string(),
        )
            .into_response()
    } else {
        (
            err.status_code(),
            [("content-type", "text/xml")],
            err.to_xml(request_id),
        )
            .into_response()
    }
}

fn parse_put_metric_data_request(
    params: &HashMap<String, String>,
    body: &[u8],
) -> PutMetricDataRequest {
    if let Ok(req) = serde_json::from_slice::<PutMetricDataRequest>(body) {
        return req;
    }

    let namespace = params.get("Namespace").cloned().unwrap_or_default();
    let mut metric_data = Vec::new();

    let mut i = 1;
    loop {
        let prefix = format!("MetricData.member.{}.", i);
        let name_key = format!("{}MetricName", prefix);
        if let Some(metric_name) = params.get(&name_key) {
            let val = params
                .get(&format!("{}Value", prefix))
                .and_then(|v| v.parse::<f64>().ok());
            let unit = params.get(&format!("{}Unit", prefix)).cloned();

            let mut dimensions = Vec::new();
            let mut j = 1;
            loop {
                let d_prefix = format!("{}Dimensions.member.{}.", prefix, j);
                let d_name = params.get(&format!("{}Name", d_prefix));
                let d_val = params.get(&format!("{}Value", d_prefix));
                if let (Some(n), Some(v)) = (d_name, d_val) {
                    dimensions.push(Dimension {
                        name: n.clone(),
                        value: v.clone(),
                    });
                    j += 1;
                } else {
                    break;
                }
            }

            metric_data.push(MetricDatum {
                metric_name: metric_name.clone(),
                dimensions: if dimensions.is_empty() {
                    None
                } else {
                    Some(dimensions)
                },
                timestamp: None,
                value: val,
                values: None,
                counts: None,
                unit,
                statistic_values: None,
                storage_resolution: None,
            });
            i += 1;
        } else {
            break;
        }
    }

    PutMetricDataRequest {
        namespace,
        metric_data,
    }
}

fn parse_list_metrics_request(params: &HashMap<String, String>, body: &[u8]) -> ListMetricsRequest {
    if let Ok(req) = serde_json::from_slice::<ListMetricsRequest>(body) {
        return req;
    }

    ListMetricsRequest {
        namespace: params.get("Namespace").cloned(),
        metric_name: params.get("MetricName").cloned(),
        dimensions: None,
        next_token: params.get("NextToken").cloned(),
        recently_active: None,
    }
}

fn parse_get_metric_data_request(
    params: &HashMap<String, String>,
    body: &[u8],
) -> GetMetricDataRequest {
    if let Ok(req) = serde_json::from_slice::<GetMetricDataRequest>(body) {
        return req;
    }

    GetMetricDataRequest {
        metric_data_queries: Vec::new(),
        start_time: chrono::Utc::now() - chrono::Duration::hours(1),
        end_time: chrono::Utc::now(),
        next_token: params.get("NextToken").cloned(),
        scan_by: None,
        max_datapoints: None,
    }
}

fn parse_get_metric_statistics_request(
    params: &HashMap<String, String>,
    body: &[u8],
) -> GetMetricStatisticsRequest {
    if let Ok(req) = serde_json::from_slice::<GetMetricStatisticsRequest>(body) {
        return req;
    }

    GetMetricStatisticsRequest {
        namespace: params.get("Namespace").cloned().unwrap_or_default(),
        metric_name: params.get("MetricName").cloned().unwrap_or_default(),
        dimensions: None,
        start_time: chrono::Utc::now() - chrono::Duration::hours(1),
        end_time: chrono::Utc::now(),
        period: params
            .get("Period")
            .and_then(|p| p.parse::<i32>().ok())
            .unwrap_or(60),
        statistics: Some(vec!["Average".to_string()]),
        extended_statistics: None,
        unit: params.get("Unit").cloned(),
    }
}

fn parse_put_metric_alarm_request(
    params: &HashMap<String, String>,
    body: &[u8],
) -> PutMetricAlarmRequest {
    if let Ok(req) = serde_json::from_slice::<PutMetricAlarmRequest>(body) {
        return req;
    }

    PutMetricAlarmRequest {
        alarm_name: params.get("AlarmName").cloned().unwrap_or_default(),
        alarm_description: params.get("AlarmDescription").cloned(),
        actions_enabled: None,
        ok_actions: None,
        alarm_actions: None,
        insufficient_data_actions: None,
        metric_name: params.get("MetricName").cloned(),
        namespace: params.get("Namespace").cloned(),
        statistic: params.get("Statistic").cloned(),
        extended_statistic: None,
        dimensions: None,
        period: params.get("Period").and_then(|p| p.parse::<i32>().ok()),
        unit: None,
        evaluation_periods: params
            .get("EvaluationPeriods")
            .and_then(|p| p.parse::<i32>().ok())
            .unwrap_or(1),
        datapoints_to_alarm: None,
        threshold: Some(
            params
                .get("Threshold")
                .and_then(|t| t.parse::<f64>().ok())
                .unwrap_or(0.0),
        ),
        comparison_operator: params
            .get("ComparisonOperator")
            .cloned()
            .unwrap_or_else(|| "GreaterThanOrEqualToThreshold".to_string()),
        treat_missing_data: None,
        evaluate_low_sample_count_percentile: None,
        metrics: None,
        tags: None,
        threshold_metric_id: None,
    }
}

fn parse_describe_alarms_request(
    params: &HashMap<String, String>,
    body: &[u8],
) -> DescribeAlarmsRequest {
    if let Ok(req) = serde_json::from_slice::<DescribeAlarmsRequest>(body) {
        return req;
    }

    DescribeAlarmsRequest {
        alarm_names: None,
        alarm_name_prefix: params.get("AlarmNamePrefix").cloned(),
        alarm_types: None,
        children_of_alarm_name: None,
        parents_of_alarm_name: None,
        state_value: params.get("StateValue").cloned(),
        action_prefix: None,
        max_records: None,
        next_token: None,
    }
}

fn parse_delete_alarms_request(
    params: &HashMap<String, String>,
    body: &[u8],
) -> DeleteAlarmsRequest {
    if let Ok(req) = serde_json::from_slice::<DeleteAlarmsRequest>(body) {
        return req;
    }

    let mut names = Vec::new();
    let mut i = 1;
    loop {
        let key = format!("AlarmNames.member.{}", i);
        if let Some(name) = params.get(&key) {
            names.push(name.clone());
            i += 1;
        } else {
            break;
        }
    }

    DeleteAlarmsRequest { alarm_names: names }
}
