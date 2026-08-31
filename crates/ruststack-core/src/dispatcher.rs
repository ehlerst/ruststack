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
            if target.starts_with("DynamoDB_20120810")
                || target.starts_with("DynamoDB")
                || target.starts_with("DynamoDB.")
            {
                return AwsService::DynamoDb;
            }
            if target.starts_with("TrentService")
                || target.starts_with("TrentService.")
                || target.starts_with("KMS")
                || target.starts_with("KMS.")
            {
                return AwsService::Kms;
            }
            if target.starts_with("Logs_20140328")
                || target.starts_with("Logs")
                || target.starts_with("Logs.")
            {
                return AwsService::Logs;
            }
            if target.starts_with("AWSIdentityManagement")
                || target.starts_with("IAM")
                || target.starts_with("IAM.")
            {
                return AwsService::Iam;
            }
            if target.starts_with("Kinesis_20131202")
                || target.starts_with("Kinesis")
                || target.starts_with("Kinesis.")
            {
                return AwsService::Kinesis;
            }
            if target.starts_with("DynamoDBStreams_20120810")
                || target.starts_with("DynamoDBStreams")
                || target.starts_with("DynamoDBStreams.")
            {
                return AwsService::DynamoDbStreams;
            }
            if target.starts_with("GraniteServiceVersion20100801")
                || target.starts_with("CloudWatch_20100801")
                || target.starts_with("CloudWatch")
                || target.starts_with("CloudWatch.")
            {
                return AwsService::CloudWatch;
            }
            if target.starts_with("SimpleEmailService")
                || target.starts_with("SES")
                || target.starts_with("SES.")
            {
                return AwsService::Ses;
            }
            if target.starts_with("AWSLambda")
                || target.starts_with("Lambda")
                || target.starts_with("Lambda.")
            {
                return AwsService::Lambda;
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
                    "dynamodb" => return AwsService::DynamoDb,
                    "kms" => return AwsService::Kms,
                    "logs" => return AwsService::Logs,
                    "iam" => return AwsService::Iam,
                    "ses" | "email" => return AwsService::Ses,
                    "kinesis" => return AwsService::Kinesis,
                    "lambda" => return AwsService::Lambda,
                    "monitoring" | "cloudwatch" => return AwsService::CloudWatch,
                    "dynamodbstreams" => return AwsService::DynamoDbStreams,
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
            if host_clean.contains(".dynamodb.") || host_clean.starts_with("dynamodb.") {
                return AwsService::DynamoDb;
            }
            if host_clean.contains(".kms.") || host_clean.starts_with("kms.") {
                return AwsService::Kms;
            }
            if host_clean.contains(".logs.") || host_clean.starts_with("logs.") {
                return AwsService::Logs;
            }
            if host_clean.contains(".iam.") || host_clean.starts_with("iam.") {
                return AwsService::Iam;
            }
            if host_clean.contains(".email.")
                || host_clean.starts_with("email.")
                || host_clean.contains(".ses.")
                || host_clean.starts_with("ses.")
            {
                return AwsService::Ses;
            }
            if host_clean.contains(".kinesis.") || host_clean.starts_with("kinesis.") {
                return AwsService::Kinesis;
            }
            if host_clean.contains(".lambda.") || host_clean.starts_with("lambda.") {
                return AwsService::Lambda;
            }
            if host_clean.contains(".monitoring.")
                || host_clean.starts_with("monitoring.")
                || host_clean.contains(".cloudwatch.")
                || host_clean.starts_with("cloudwatch.")
            {
                return AwsService::CloudWatch;
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
                && !host_clean.starts_with("dynamodb")
                && !host_clean.starts_with("kms")
                && !host_clean.starts_with("logs")
                && !host_clean.starts_with("iam")
                && !host_clean.starts_with("ses")
                && !host_clean.starts_with("email")
                && !host_clean.starts_with("kinesis")
                && !host_clean.starts_with("lambda")
                && !host_clean.starts_with("monitoring")
                && !host_clean.starts_with("cloudwatch")
            {
                return AwsService::S3;
            }
        }

        // 5. Check URL path patterns
        if path.starts_with("/2015-03-31/functions") {
            return AwsService::Lambda;
        }

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
            if first == "dynamodb" {
                return AwsService::DynamoDb;
            }
            if first == "ses" {
                return AwsService::Ses;
            }
            if first == "kinesis" {
                return AwsService::Kinesis;
            }
            if first == "lambda" {
                return AwsService::Lambda;
            }
            if first == "cloudwatch" || first == "monitoring" {
                return AwsService::CloudWatch;
            }
        }

        // 6. Check Query Parameters for SQS, SNS, STS, IAM, SES, CloudWatch Actions
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
                        if is_iam_action(&v) {
                            return AwsService::Iam;
                        }
                        if is_ses_action(&v) {
                            return AwsService::Ses;
                        }
                        if is_cloudwatch_action(&v) {
                            return AwsService::CloudWatch;
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
                            if is_iam_action(&v) {
                                return AwsService::Iam;
                            }
                            if is_ses_action(&v) {
                                return AwsService::Ses;
                            }
                            if is_cloudwatch_action(&v) {
                                return AwsService::CloudWatch;
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

fn is_iam_action(action: &str) -> bool {
    matches!(
        action,
        "CreateRole"
            | "GetRole"
            | "DeleteRole"
            | "ListRoles"
            | "UpdateRole"
            | "CreatePolicy"
            | "GetPolicy"
            | "DeletePolicy"
            | "ListPolicies"
            | "AttachRolePolicy"
            | "DetachRolePolicy"
            | "ListAttachedRolePolicies"
            | "PutRolePolicy"
            | "GetRolePolicy"
            | "DeleteRolePolicy"
            | "ListRolePolicies"
            | "CreateUser"
            | "GetUser"
            | "DeleteUser"
            | "ListUsers"
            | "CreateAccessKey"
            | "ListAccessKeys"
            | "DeleteAccessKey"
            | "UpdateAccessKey"
            | "PutUserPolicy"
            | "GetUserPolicy"
            | "DeleteUserPolicy"
            | "ListUserPolicies"
    )
}

fn is_ses_action(action: &str) -> bool {
    matches!(
        action,
        "SendEmail"
            | "SendRawEmail"
            | "SendBulkTemplatedEmail"
            | "SendCustomVerificationEmail"
            | "VerifyEmailIdentity"
            | "VerifyDomainIdentity"
            | "VerifyDomainDkim"
            | "DeleteIdentity"
            | "ListIdentities"
            | "GetIdentityVerificationAttributes"
            | "GetSendQuota"
            | "GetSendStatistics"
            | "GetAccount"
    )
}

fn is_cloudwatch_action(action: &str) -> bool {
    matches!(
        action,
        "PutMetricData"
            | "GetMetricData"
            | "GetMetricStatistics"
            | "ListMetrics"
            | "PutMetricAlarm"
            | "DescribeAlarms"
            | "DescribeAlarmsForMetric"
            | "DeleteAlarms"
            | "SetAlarmState"
            | "EnableAlarmActions"
            | "DisableAlarmActions"
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
    fn test_dynamodb_dispatch() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-amz-target",
            HeaderValue::from_static("DynamoDB_20120810.PutItem"),
        );
        let uri: Uri = "/".parse().unwrap();
        assert_eq!(
            Dispatcher::classify_request(&Method::POST, &uri, &headers, None),
            AwsService::DynamoDb
        );
    }
}
