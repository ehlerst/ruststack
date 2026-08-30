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

        // 2. Check x-amz-target header (used by SQS JSON 1.0, DynamoDB, etc.)
        if let Some(target) = headers.get("x-amz-target").and_then(|v| v.to_str().ok()) {
            if target.starts_with("AmazonSQS") || target.starts_with("AmazonSQS.") {
                return AwsService::Sqs;
            }
        }

        // 3. Check Authorization header (AWS SigV4 credential scope: .../us-east-1/<service>/aws4_request)
        if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
            if let Some(service) = extract_service_from_auth(auth) {
                match service {
                    "s3" => return AwsService::S3,
                    "sqs" => return AwsService::Sqs,
                    _ => {}
                }
            }
        }

        // 4. Check Host header for virtual hosting (e.g., mybucket.s3.localhost:4566 or sqs.us-east-1.localhost)
        if let Some(host) = headers.get("host").and_then(|v| v.to_str().ok()) {
            let host_clean = host.split(':').next().unwrap_or(host);
            if host_clean.contains(".s3.") || host_clean.starts_with("s3.") {
                return AwsService::S3;
            }
            if host_clean.contains(".sqs.") || host_clean.starts_with("sqs.") {
                return AwsService::Sqs;
            }
            // Check for subdomain style like bucket.localhost
            if host_clean.ends_with(".localhost") && !host_clean.starts_with("localhost") {
                return AwsService::S3;
            }
        }

        // 5. Check URL path patterns
        // SQS Queue URLs are formatted as: /000000000000/queue-name or /queue/queue-name
        let segments: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        if !segments.is_empty() {
            let first = segments[0];
            // 12-digit AWS account number
            if first.len() == 12 && first.chars().all(|c| c.is_ascii_digit()) {
                return AwsService::Sqs;
            }
            if first == "queue" || first == "queues" {
                return AwsService::Sqs;
            }
        }

        // 6. Check Query Parameters for SQS Actions
        if let Some(query) = uri.query() {
            if query.contains("Action=") {
                let params = form_urlencoded::parse(query.as_bytes());
                for (k, v) in params {
                    if k == "Action" && is_sqs_action(&v) {
                        return AwsService::Sqs;
                    }
                }
            }
        }

        // 7. Check Form Body peek for SQS Action (POST urlencoded)
        if let Some(body) = body_peek {
            if let Ok(body_str) = std::str::from_utf8(body) {
                if body_str.contains("Action=") {
                    let params = form_urlencoded::parse(body);
                    for (k, v) in params {
                        if k == "Action" && is_sqs_action(&v) {
                            return AwsService::Sqs;
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
    fn test_auth_header_dispatch() {
        let uri: Uri = "/test".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static(
                "AWS4-HMAC-SHA256 Credential=TEST/20260830/us-east-1/s3/aws4_request, SignedHeaders=host, Signature=abc",
            ),
        );
        assert_eq!(
            Dispatcher::classify_request(&Method::GET, &uri, &headers, None),
            AwsService::S3
        );

        headers.insert(
            "authorization",
            HeaderValue::from_static(
                "AWS4-HMAC-SHA256 Credential=TEST/20260830/us-east-1/sqs/aws4_request, SignedHeaders=host, Signature=abc",
            ),
        );
        assert_eq!(
            Dispatcher::classify_request(&Method::POST, &uri, &headers, None),
            AwsService::Sqs
        );
    }

    #[test]
    fn test_target_header_sqs() {
        let uri: Uri = "/".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-amz-target",
            HeaderValue::from_static("AmazonSQS.SendMessage"),
        );
        assert_eq!(
            Dispatcher::classify_request(&Method::POST, &uri, &headers, None),
            AwsService::Sqs
        );
    }

    #[test]
    fn test_sqs_account_queue_path() {
        let uri: Uri = "/000000000000/my-test-queue".parse().unwrap();
        let headers = HeaderMap::new();
        assert_eq!(
            Dispatcher::classify_request(&Method::POST, &uri, &headers, None),
            AwsService::Sqs
        );
    }

    #[test]
    fn test_sqs_query_action() {
        let uri: Uri = "/?Action=CreateQueue&QueueName=my-queue".parse().unwrap();
        let headers = HeaderMap::new();
        assert_eq!(
            Dispatcher::classify_request(&Method::GET, &uri, &headers, None),
            AwsService::Sqs
        );
    }

    #[test]
    fn test_s3_path_style() {
        let uri: Uri = "/my-bucket/my-key.txt".parse().unwrap();
        let headers = HeaderMap::new();
        assert_eq!(
            Dispatcher::classify_request(&Method::GET, &uri, &headers, None),
            AwsService::S3
        );
    }
}
