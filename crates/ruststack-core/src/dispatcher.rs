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

        // 2. Check x-amz-target header (SQS, EventBridge, SNS JSON, DynamoDB, etc.)
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
        }

        // 3. Check Authorization header (AWS SigV4 credential scope: .../us-east-1/<service>/aws4_request)
        if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
            if let Some(service) = extract_service_from_auth(auth) {
                match service {
                    "s3" => return AwsService::S3,
                    "sqs" => return AwsService::Sqs,
                    "sns" => return AwsService::Sns,
                    "events" | "eventbridge" => return AwsService::EventBridge,
                    _ => {}
                }
            }
        }

        // 4. Check Host header for virtual hosting (e.g. sns.localhost, events.localhost, bucket.s3.localhost)
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
            // Check for S3 bucket subdomain style like bucket.localhost
            if host_clean.ends_with(".localhost")
                && !host_clean.starts_with("localhost")
                && !host_clean.starts_with("sns")
                && !host_clean.starts_with("sqs")
                && !host_clean.starts_with("events")
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
        }

        // 6. Check Query Parameters for SQS and SNS Actions
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
    fn test_sns_dispatch() {
        let uri: Uri = "/?Action=CreateTopic&Name=my-topic".parse().unwrap();
        let headers = HeaderMap::new();
        assert_eq!(
            Dispatcher::classify_request(&Method::POST, &uri, &headers, None),
            AwsService::Sns
        );

        let mut headers_auth = HeaderMap::new();
        headers_auth.insert(
            "authorization",
            HeaderValue::from_static(
                "AWS4-HMAC-SHA256 Credential=TEST/20260830/us-east-1/sns/aws4_request, SignedHeaders=host, Signature=abc",
            ),
        );
        let uri_any: Uri = "/".parse().unwrap();
        assert_eq!(
            Dispatcher::classify_request(&Method::POST, &uri_any, &headers_auth, None),
            AwsService::Sns
        );
    }

    #[test]
    fn test_eventbridge_dispatch() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-amz-target",
            HeaderValue::from_static("AWSEvents.PutEvents"),
        );
        let uri: Uri = "/".parse().unwrap();
        assert_eq!(
            Dispatcher::classify_request(&Method::POST, &uri, &headers, None),
            AwsService::EventBridge
        );
    }
}
