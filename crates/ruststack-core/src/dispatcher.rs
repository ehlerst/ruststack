use crate::types::AwsService;
use http::{HeaderMap, Method, Uri};

pub struct Dispatcher;

impl Dispatcher {
    pub fn classify_request(
        _method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body_peek: Option<&[u8]>,
    ) -> AwsService {
        let path = uri.path();

        // 1. Internal endpoints
        if path == "/_ruststack/health"
            || path == "/_ruststack/info"
            || path == "/health"
            || path == "/ping"
        {
            return AwsService::Internal;
        }

        // 2. Check x-amz-target header
        if let Some(target) = headers.get("x-amz-target").and_then(|v| v.to_str().ok()) {
            if target.starts_with("AmazonSQS") || target.starts_with("AmazonSQS.") {
                return AwsService::Sqs;
            }
            if target.starts_with("AmazonSNS") || target.starts_with("AmazonSNS.") {
                return AwsService::Sns;
            }
            if target.starts_with("AWSEvents")
                || target.starts_with("AmazonEventBridge")
                || target.starts_with("EventBridge")
            {
                return AwsService::EventBridge;
            }
            if target.starts_with("AmazonSSM") || target.starts_with("AmazonSSM.") {
                return AwsService::Ssm;
            }
            if target.starts_with("secretsmanager") || target.starts_with("secretsmanager.") {
                return AwsService::SecretsManager;
            }
            if target.starts_with("AWSSecurityTokenServiceV20110615")
                || target.starts_with("STS")
                || target.starts_with("STS.")
            {
                return AwsService::Sts;
            }
        }

        // 3. Check Authorization header (AWS SigV4 credential scope: .../us-east-1/<service>/aws4_request)
        if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
            if let Some(service) = extract_service_from_auth(auth) {
                match service {
                    "s3" => return AwsService::S3,
                    "sqs" => return AwsService::Sqs,
                    "sns" => return AwsService::Sns,
                    "events" | "eventbridge" => return AwsService::EventBridge,
                    "ssm" => return AwsService::Ssm,
                    "secretsmanager" => return AwsService::SecretsManager,
                    "sts" => return AwsService::Sts,
                    _ => {}
                }
            }
        }

        // 4. Check Host header for virtual hosting
        if let Some(host) = headers.get("host").and_then(|v| v.to_str().ok()) {
            let host_clean = host.split(':').next().unwrap_or(host);
            if host_clean.contains(".s3.") || host_clean.starts_with("s3.") {
                return AwsService::S3;
            }
            if host_clean.contains(".sqs.") || host_clean.starts_with("sqs.") {
                return AwsService::Sqs;
            }
            if host_clean.contains(".sns.") || host_clean.starts_with("sns.") {
                return AwsService::Sns;
            }
            if host_clean.contains(".events.")
                || host_clean.starts_with("events.")
                || host_clean.contains(".eventbridge.")
                || host_clean.starts_with("eventbridge.")
            {
                return AwsService::EventBridge;
            }
            if host_clean.contains(".ssm.") || host_clean.starts_with("ssm.") {
                return AwsService::Ssm;
            }
            if host_clean.contains(".secretsmanager.") || host_clean.starts_with("secretsmanager.")
            {
                return AwsService::SecretsManager;
            }
            if host_clean.contains(".sts.") || host_clean.starts_with("sts.") {
                return AwsService::Sts;
            }
            // Check for S3 bucket subdomain style like bucket.localhost
            if host_clean.ends_with(".localhost")
                && !host_clean.starts_with("localhost")
                && !host_clean.starts_with("sns")
                && !host_clean.starts_with("sqs")
                && !host_clean.starts_with("events")
                && !host_clean.starts_with("ssm")
                && !host_clean.starts_with("secretsmanager")
                && !host_clean.starts_with("sts")
            {
                return AwsService::S3;
            }
        }

        // 5. Check URL path patterns
        let segments: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        if !segments.is_empty() {
            let first = segments[0];
            // 12-digit AWS account number for SQS queue URL
            if first.len() == 12 && first.chars().all(|c| c.is_ascii_digit()) {
                return AwsService::Sqs;
            }
            if first == "queue" || first == "queues" {
                return AwsService::Sqs;
            }
            if first == "sns" || first == "topics" {
                return AwsService::Sns;
            }
            if first == "events" || first == "eventbridge" {
                return AwsService::EventBridge;
            }
            if first == "ssm" {
                return AwsService::Ssm;
            }
            if first == "secretsmanager" {
                return AwsService::SecretsManager;
            }
            if first == "sts" {
                return AwsService::Sts;
            }
        }

        // 6. Check Query Parameters for SQS, SNS, and STS Actions
        if let Some(query) = uri.query() {
            if query.contains("Action=") {
                let params = form_urlencoded::parse(query.as_bytes());
                for (k, v) in params {
                    if k == "Action" {
                        if is_sqs_action(&v) {
                            return AwsService::Sqs;
                        }
                        if is_sns_action(&v) {
                            return AwsService::Sns;
                        }
                        if is_sts_action(&v) {
                            return AwsService::Sts;
                        }
                    }
                }
            }
        }

        // 7. Check Form Body peek for Actions (POST urlencoded)
        if let Some(body) = body_peek {
            if let Ok(body_str) = std::str::from_utf8(body) {
                if body_str.contains("Action=") {
                    let params = form_urlencoded::parse(body);
                    for (k, v) in params {
                        if k == "Action" {
                            if is_sqs_action(&v) {
                                return AwsService::Sqs;
                            }
                            if is_sns_action(&v) {
                                return AwsService::Sns;
                            }
                            if is_sts_action(&v) {
                                return AwsService::Sts;
                            }
                        }
                    }
                }
            }
        }

        // Default to S3 for REST path-style operations (e.g. GET /bucket, PUT /bucket/key, etc.)
        AwsService::S3
    }
}

fn extract_service_from_auth(auth: &str) -> Option<&str> {
    // AWS4-HMAC-SHA256 Credential=AKIA.../20260830/us-east-1/s3/aws4_request
    if let Some(cred_start) = auth.find("Credential=") {
        let cred = &auth[cred_start + 11..];
        let cred_val = cred.split(',').next()?.trim();
        let parts: Vec<&str> = cred_val.split('/').collect();
        if parts.len() >= 5 && parts[parts.len() - 1] == "aws4_request" {
            return Some(parts[parts.len() - 2]);
        }
    }
    None
}

fn is_sqs_action(action: &str) -> bool {
    matches!(
        action,
        "CreateQueue"
            | "DeleteQueue"
            | "ListQueues"
            | "GetQueueUrl"
            | "GetQueueAttributes"
            | "SetQueueAttributes"
            | "PurgeQueue"
            | "SendMessage"
            | "SendMessageBatch"
            | "ReceiveMessage"
            | "DeleteMessage"
            | "DeleteMessageBatch"
            | "ChangeMessageVisibility"
            | "ChangeMessageVisibilityBatch"
            | "ListDeadLetterSourceQueues"
            | "AddPermission"
            | "RemovePermission"
            | "TagQueue"
            | "UntagQueue"
            | "ListQueueTags"
    )
}

fn is_sns_action(action: &str) -> bool {
    matches!(
        action,
        "CreateTopic"
            | "DeleteTopic"
            | "ListTopics"
            | "GetTopicAttributes"
            | "SetTopicAttributes"
            | "Subscribe"
            | "Unsubscribe"
            | "ListSubscriptions"
            | "ListSubscriptionsByTopic"
            | "Publish"
            | "PublishBatch"
            | "ConfirmSubscription"
            | "GetSubscriptionAttributes"
            | "SetSubscriptionAttributes"
            | "TagResource"
            | "UntagResource"
            | "ListTagsForResource"
    )
}

fn is_sts_action(action: &str) -> bool {
    matches!(
        action,
        "GetCallerIdentity"
            | "AssumeRole"
            | "AssumeRoleWithWebIdentity"
            | "AssumeRoleWithSAML"
            | "GetSessionToken"
            | "GetFederationToken"
            | "DecodeAuthorizationMessage"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{HeaderMap, HeaderValue, Method, Uri};

    #[test]
    fn test_internal_dispatch() {
        let uri: Uri = "/_ruststack/health".parse().unwrap();
        let headers = HeaderMap::new();
        assert_eq!(
            Dispatcher::classify_request(&Method::GET, &uri, &headers, None),
            AwsService::Internal
        );
    }

    #[test]
    fn test_ssm_dispatch() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-amz-target",
            HeaderValue::from_static("AmazonSSM.GetParameter"),
        );
        let uri: Uri = "/".parse().unwrap();
        assert_eq!(
            Dispatcher::classify_request(&Method::POST, &uri, &headers, None),
            AwsService::Ssm
        );
    }

    #[test]
    fn test_secretsmanager_dispatch() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-amz-target",
            HeaderValue::from_static("secretsmanager.GetSecretValue"),
        );
        let uri: Uri = "/".parse().unwrap();
        assert_eq!(
            Dispatcher::classify_request(&Method::POST, &uri, &headers, None),
            AwsService::SecretsManager
        );
    }

    #[test]
    fn test_sts_dispatch() {
        let uri: Uri = "/?Action=GetCallerIdentity&Version=2011-06-15"
            .parse()
            .unwrap();
        let headers = HeaderMap::new();
        assert_eq!(
            Dispatcher::classify_request(&Method::POST, &uri, &headers, None),
            AwsService::Sts
        );
    }
}
