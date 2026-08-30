use crate::types::AwsService;
use axum::body::Body;
use axum::http::{Response, StatusCode};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosRule {
    #[serde(default = "generate_rule_id")]
    pub id: String,
    pub name: Option<String>,
    pub service: Option<String>,      // e.g. "s3", "sqs", "dynamodb"
    pub action: Option<String>,       // e.g. "PutItem", "SendMessage", "GetObject"
    pub path_pattern: Option<String>, // substring match on path
    #[serde(default = "default_probability")]
    pub probability: f64, // 0.0 to 1.0 (default 1.0)
    pub error_status: Option<u16>,    // e.g. 500, 503, 400, 429
    pub error_code: Option<String>,   // e.g. "ProvisionedThroughputExceededException", "SlowDown"
    pub error_message: Option<String>, // Custom error message
    pub latency_ms: Option<u64>,      // Injected fixed latency
    pub latency_jitter_ms: Option<u64>, // ± Jitter range
    pub limit_times: Option<u32>,     // Trigger at most N times then expire
    #[serde(default)]
    pub times_triggered: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn generate_rule_id() -> String {
    format!("chaos-{}", &uuid::Uuid::new_v4().to_string()[..8])
}

fn default_probability() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

#[derive(Debug)]
struct InternalChaosRule {
    id: String,
    name: Option<String>,
    service: Option<String>,
    action: Option<String>,
    path_pattern: Option<String>,
    probability: f64,
    error_status: Option<u16>,
    error_code: Option<String>,
    error_message: Option<String>,
    latency_ms: Option<u64>,
    latency_jitter_ms: Option<u64>,
    limit_times: Option<u32>,
    times_triggered: AtomicU32,
    enabled: bool,
}

impl From<&ChaosRule> for InternalChaosRule {
    fn from(r: &ChaosRule) -> Self {
        Self {
            id: r.id.clone(),
            name: r.name.clone(),
            service: r.service.as_ref().map(|s| s.to_lowercase()),
            action: r.action.as_ref().map(|a| a.to_lowercase()),
            path_pattern: r.path_pattern.clone(),
            probability: r.probability.clamp(0.0, 1.0),
            error_status: r.error_status,
            error_code: r.error_code.clone(),
            error_message: r.error_message.clone(),
            latency_ms: r.latency_ms,
            latency_jitter_ms: r.latency_jitter_ms,
            limit_times: r.limit_times,
            times_triggered: AtomicU32::new(r.times_triggered),
            enabled: r.enabled,
        }
    }
}

impl From<&InternalChaosRule> for ChaosRule {
    fn from(r: &InternalChaosRule) -> Self {
        Self {
            id: r.id.clone(),
            name: r.name.clone(),
            service: r.service.clone(),
            action: r.action.clone(),
            path_pattern: r.path_pattern.clone(),
            probability: r.probability,
            error_status: r.error_status,
            error_code: r.error_code.clone(),
            error_message: r.error_message.clone(),
            latency_ms: r.latency_ms,
            latency_jitter_ms: r.latency_jitter_ms,
            limit_times: r.limit_times,
            times_triggered: r.times_triggered.load(Ordering::Relaxed),
            enabled: r.enabled,
        }
    }
}

#[derive(Debug)]
pub enum ChaosDecision {
    PassThrough {
        latency_ms: Option<u64>,
    },
    InjectError {
        status: StatusCode,
        code: String,
        message: String,
        latency_ms: Option<u64>,
    },
}

#[derive(Debug)]
pub struct ChaosEngine {
    enabled: AtomicBool,
    rules: RwLock<Vec<Arc<InternalChaosRule>>>,
}

impl Default for ChaosEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ChaosEngine {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            rules: RwLock::new(Vec::new()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn add_rule(&self, mut rule: ChaosRule) -> String {
        if rule.id.is_empty() {
            rule.id = generate_rule_id();
        }
        let id = rule.id.clone();
        let internal = Arc::new(InternalChaosRule::from(&rule));
        self.rules.write().push(internal);
        id
    }

    pub fn get_rules(&self) -> Vec<ChaosRule> {
        self.rules
            .read()
            .iter()
            .map(|r| ChaosRule::from(r.as_ref()))
            .collect()
    }

    pub fn remove_rule(&self, id: &str) -> bool {
        let mut rules = self.rules.write();
        if let Some(pos) = rules.iter().position(|r| r.id == id) {
            rules.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn clear_rules(&self) {
        self.rules.write().clear();
    }

    pub fn evaluate(&self, service: AwsService, action: Option<&str>, path: &str) -> ChaosDecision {
        if !self.is_enabled() {
            return ChaosDecision::PassThrough { latency_ms: None };
        }

        let rules = self.rules.read();
        if rules.is_empty() {
            return ChaosDecision::PassThrough { latency_ms: None };
        }

        let svc_str = match service {
            AwsService::S3 => "s3",
            AwsService::Sqs => "sqs",
            AwsService::Sns => "sns",
            AwsService::EventBridge => "eventbridge",
            AwsService::Ssm => "ssm",
            AwsService::SecretsManager => "secretsmanager",
            AwsService::Sts => "sts",
            AwsService::DynamoDb => "dynamodb",
            _ => "unknown",
        };

        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            // Check service match
            if let Some(ref target_svc) = rule.service {
                if target_svc != svc_str && target_svc != "all" {
                    continue;
                }
            }

            // Check action match
            if let Some(ref target_act) = rule.action {
                let matches_action = if let Some(req_act) = action {
                    req_act.to_lowercase().contains(target_act)
                        || target_act.contains(&req_act.to_lowercase())
                } else {
                    false
                };
                if !matches_action {
                    continue;
                }
            }

            // Check path pattern match
            if let Some(ref pattern) = rule.path_pattern {
                if !path.contains(pattern) {
                    continue;
                }
            }

            // Check limit times
            let current_count = rule.times_triggered.load(Ordering::Relaxed);
            if let Some(limit) = rule.limit_times {
                if current_count >= limit {
                    continue;
                }
            }

            // Check probability
            if rule.probability < 1.0 {
                let roll = fastrand::f64();
                if roll > rule.probability {
                    continue;
                }
            }

            // Matched! Increment trigger counter
            rule.times_triggered.fetch_add(1, Ordering::Relaxed);

            // Compute latency
            let latency_ms = if let Some(base) = rule.latency_ms {
                if let Some(jitter) = rule.latency_jitter_ms {
                    if jitter > 0 {
                        let jitter_val = fastrand::u64(0..=(jitter * 2));
                        let calculated = base.saturating_sub(jitter) + jitter_val;
                        Some(calculated)
                    } else {
                        Some(base)
                    }
                } else {
                    Some(base)
                }
            } else {
                None
            };

            // Check if error is configured
            if rule.error_status.is_some() || rule.error_code.is_some() {
                let status_code = rule
                    .error_status
                    .and_then(|s| StatusCode::from_u16(s).ok())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                let code = rule
                    .error_code
                    .clone()
                    .unwrap_or_else(|| "InternalError".to_string());

                let message = rule
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Chaos injection simulated error by RustStack".to_string());

                return ChaosDecision::InjectError {
                    status: status_code,
                    code,
                    message,
                    latency_ms,
                };
            } else {
                return ChaosDecision::PassThrough { latency_ms };
            }
        }

        ChaosDecision::PassThrough { latency_ms: None }
    }

    pub fn format_error_response(
        service: AwsService,
        status: StatusCode,
        code: &str,
        message: &str,
        request_id: &str,
    ) -> Response<Body> {
        match service {
            AwsService::S3 => {
                let xml = format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>{}</Code>
    <Message>{}</Message>
    <RequestId>{}</RequestId>
</Error>"#,
                    code, message, request_id
                );
                Response::builder()
                    .status(status)
                    .header("content-type", "application/xml")
                    .header("x-amz-request-id", request_id)
                    .body(Body::from(xml))
                    .unwrap()
            }
            AwsService::DynamoDb => {
                let json_body = serde_json::json!({
                    "__type": format!("com.amazonaws.dynamodb.v20120810#{}", code),
                    "message": message
                });
                Response::builder()
                    .status(status)
                    .header("content-type", "application/x-amz-json-1.0")
                    .header("x-amzn-requestid", request_id)
                    .body(Body::from(json_body.to_string()))
                    .unwrap()
            }
            AwsService::Ssm
            | AwsService::SecretsManager
            | AwsService::EventBridge
            | AwsService::Sts => {
                let json_body = serde_json::json!({
                    "__type": code,
                    "message": message
                });
                Response::builder()
                    .status(status)
                    .header("content-type", "application/x-amz-json-1.1")
                    .header("x-amzn-requestid", request_id)
                    .body(Body::from(json_body.to_string()))
                    .unwrap()
            }
            AwsService::Sqs | AwsService::Sns => {
                let xml = format!(
                    r#"<ErrorResponse>
    <Error>
        <Type>Receiver</Type>
        <Code>{}</Code>
        <Message>{}</Message>
    </Error>
    <RequestId>{}</RequestId>
</ErrorResponse>"#,
                    code, message, request_id
                );
                Response::builder()
                    .status(status)
                    .header("content-type", "application/xml")
                    .header("x-amzn-requestid", request_id)
                    .body(Body::from(xml))
                    .unwrap()
            }
            _ => {
                let json_body = serde_json::json!({
                    "error": code,
                    "message": message
                });
                Response::builder()
                    .status(status)
                    .header("content-type", "application/json")
                    .body(Body::from(json_body.to_string()))
                    .unwrap()
            }
        }
    }
}
