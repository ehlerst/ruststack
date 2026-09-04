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
            if target.starts_with("AWSCognitoIdentityProviderService")
                || target.starts_with("AWSCognitoIdentityProviderService.")
                || target.starts_with("Cognito")
                || target.starts_with("Cognito.")
            {
                return AwsService::Cognito;
            }
            if target.starts_with("AWSStepFunctions")
                || target.starts_with("AWSStepFunctions.")
                || target.starts_with("StepFunctions")
                || target.starts_with("StepFunctions.")
            {
                return AwsService::StepFunctions;
            }
            if target.starts_with("AmazonEC2ContainerRegistry_V20150921")
                || target.starts_with("AmazonEC2ContainerRegistry_V20150921.")
                || target.starts_with("ECR")
                || target.starts_with("ECR.")
            {
                return AwsService::Ecr;
            }
            if target.starts_with("AmazonEC2ContainerServiceV20141113")
                || target.starts_with("AmazonEC2ContainerServiceV20141113.")
                || target.starts_with("ECS")
                || target.starts_with("ECS.")
            {
                return AwsService::Ecs;
            }
            if target.starts_with("AmazonAthena")
                || target.starts_with("AmazonAthena.")
                || target.starts_with("Athena")
                || target.starts_with("Athena.")
            {
                return AwsService::Athena;
            }
            if target.starts_with("AmazonBedrockControlPlaneService")
                || target.starts_with("Bedrock")
            {
                return AwsService::Bedrock;
            }
            if target.starts_with("CertificateManager") || target.starts_with("ACM") {
                return AwsService::Acm;
            }
            if target.starts_with("AWSWAF_20190729") || target.starts_with("WAF") {
                return AwsService::WafV2;
            }
            if target.starts_with("AWSOrganizationsV20161128") || target.starts_with("Organizations") {
                return AwsService::Organizations;
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
                    "cognito-idp" | "cognito" => return AwsService::Cognito,
                    "apigateway" => return AwsService::ApiGateway,
                    "route53" => return AwsService::Route53,
                    "states" | "stepfunctions" => return AwsService::StepFunctions,
                    "cloudformation" => return AwsService::CloudFormation,
                    "ecr" => return AwsService::Ecr,
                    "ecs" => return AwsService::Ecs,
                    "ec2" => return AwsService::Ec2,
                    "elasticloadbalancing" | "elbv2" | "elb" => return AwsService::Elbv2,
                    "bedrock" | "bedrock-runtime" => return AwsService::Bedrock,
                    "es" | "opensearch" => return AwsService::OpenSearch,
                    "athena" => return AwsService::Athena,
                    "rds" => return AwsService::Rds,
                    "elasticache" => return AwsService::ElastiCache,
                    "redshift" => return AwsService::Redshift,
                    "acm" => return AwsService::Acm,
                    "wafv2" | "waf-regional" | "waf" => return AwsService::WafV2,
                    "organizations" => return AwsService::Organizations,
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
        if path.ends_with("/.well-known/jwks.json") || path == "/.well-known/jwks.json" {
            return AwsService::Cognito;
        }
        if path.starts_with("/restapis") || path.starts_with("/execute-api") {
            return AwsService::ApiGateway;
        }
        if path.starts_with("/2013-04-01") || path.starts_with("/hostedzone") {
            return AwsService::Route53;
        }

        let segments: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        if !segments.is_empty() {
            let first = segments[0];
            // 10-hex digit API Gateway ID e.g. /{api_id}/{stage}/...
            if segments.len() >= 2
                && first.len() == 10
                && first.chars().all(|c| c.is_ascii_hexdigit())
            {
                return AwsService::ApiGateway;
            }
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
            if first == "cognito" || first == "cognito-idp" {
                return AwsService::Cognito;
            }
            if first == "route53" {
                return AwsService::Route53;
            }
            if first == "states" || first == "stepfunctions" {
                return AwsService::StepFunctions;
            }
            if first == "cloudformation" {
                return AwsService::CloudFormation;
            }
            if first == "ecr" {
                return AwsService::Ecr;
            }
            if first == "ecs" {
                return AwsService::Ecs;
            }
            if first == "ec2" {
                return AwsService::Ec2;
            }
            if first == "elasticloadbalancing" || first == "elbv2" || first == "elb" {
                return AwsService::Elbv2;
            }
            if first == "foundation-models" || first == "model" || first == "bedrock" {
                return AwsService::Bedrock;
            }
            if first == "opensearch"
                || (first == "2021-01-01" && path.contains("/opensearch/"))
                || (first == "2015-01-01" && path.contains("/es/"))
            {
                return AwsService::OpenSearch;
            }
            if first == "athena" {
                return AwsService::Athena;
            }
            if first == "rds" {
                return AwsService::Rds;
            }
            if first == "elasticache" {
                return AwsService::ElastiCache;
            }
            if first == "redshift" {
                return AwsService::Redshift;
            }
            if first == "acm" {
                return AwsService::Acm;
            }
            if first == "wafv2" || first == "waf" {
                return AwsService::WafV2;
            }
            if first == "organizations" {
                return AwsService::Organizations;
            }
        }

        // 6. Check Query Parameters for SQS, SNS, STS, IAM, SES, CloudWatch, CloudFormation, EC2, ELBv2, RDS, ElastiCache, Redshift Actions
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
                        if is_cloudformation_action(&v) {
                            return AwsService::CloudFormation;
                        }
                        if is_ec2_action(&v) {
                            return AwsService::Ec2;
                        }
                        if is_elbv2_action(&v) {
                            return AwsService::Elbv2;
                        }
                        if is_rds_action(&v) {
                            return AwsService::Rds;
                        }
                        if is_elasticache_action(&v) {
                            return AwsService::ElastiCache;
                        }
                        if is_redshift_action(&v) {
                            return AwsService::Redshift;
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
                            if is_cloudformation_action(&v) {
                                return AwsService::CloudFormation;
                            }
                            if is_ec2_action(&v) {
                                return AwsService::Ec2;
                            }
                            if is_elbv2_action(&v) {
                                return AwsService::Elbv2;
                            }
                            if is_rds_action(&v) {
                                return AwsService::Rds;
                            }
                            if is_elasticache_action(&v) {
                                return AwsService::ElastiCache;
                            }
                            if is_redshift_action(&v) {
                                return AwsService::Redshift;
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

fn is_cloudformation_action(action: &str) -> bool {
    matches!(
        action,
        "CreateStack"
            | "DescribeStacks"
            | "DescribeStackResources"
            | "DescribeStackEvents"
            | "GetTemplate"
            | "UpdateStack"
            | "DeleteStack"
            | "ListStacks"
            | "CreateChangeSet"
            | "DescribeChangeSet"
            | "ExecuteChangeSet"
            | "DeleteChangeSet"
    )
}

fn is_ec2_action(action: &str) -> bool {
    matches!(
        action,
        "CreateVpc"
            | "DescribeVpcs"
            | "DeleteVpc"
            | "CreateSubnet"
            | "DescribeSubnets"
            | "DeleteSubnet"
            | "CreateSecurityGroup"
            | "DescribeSecurityGroups"
            | "DeleteSecurityGroup"
            | "AuthorizeSecurityGroupIngress"
            | "RevokeSecurityGroupIngress"
            | "AuthorizeSecurityGroupEgress"
            | "RevokeSecurityGroupEgress"
            | "CreateKeyPair"
            | "DescribeKeyPairs"
            | "DeleteKeyPair"
            | "RunInstances"
            | "DescribeInstances"
            | "TerminateInstances"
            | "StartInstances"
            | "StopInstances"
            | "DescribeAvailabilityZones"
            | "DescribeImages"
            | "DescribeTags"
    )
}

fn is_elbv2_action(action: &str) -> bool {
    matches!(
        action,
        "CreateLoadBalancer"
            | "DescribeLoadBalancers"
            | "DeleteLoadBalancer"
            | "CreateTargetGroup"
            | "DescribeTargetGroups"
            | "DeleteTargetGroup"
            | "RegisterTargets"
            | "DeregisterTargets"
            | "DescribeTargetHealth"
            | "CreateListener"
            | "DescribeListeners"
            | "DeleteListener"
            | "CreateRule"
            | "DescribeRules"
            | "DeleteRule"
            | "AddTags"
            | "RemoveTags"
            | "DescribeTags"
    )
}

fn is_rds_action(action: &str) -> bool {
    matches!(
        action,
        "CreateDBInstance"
            | "DescribeDBInstances"
            | "DeleteDBInstance"
            | "CreateDBCluster"
            | "DescribeDBClusters"
            | "DeleteDBCluster"
            | "CreateDBSnapshot"
            | "DescribeDBSnapshots"
            | "DeleteDBSnapshot"
            | "ModifyDBInstance"
            | "RebootDBInstance"
            | "AddTagsToResource"
            | "ListTagsForResource"
            | "RemoveTagsFromResource"
    )
}

fn is_elasticache_action(action: &str) -> bool {
    matches!(
        action,
        "CreateCacheCluster"
            | "DescribeCacheClusters"
            | "DeleteCacheCluster"
            | "CreateReplicationGroup"
            | "DescribeReplicationGroups"
            | "DeleteReplicationGroup"
            | "ModifyCacheCluster"
            | "RebootCacheCluster"
            | "AddTagsToResource"
            | "ListTagsForResource"
            | "RemoveTagsFromResource"
    )
}

fn is_redshift_action(action: &str) -> bool {
    matches!(
        action,
        "CreateCluster"
            | "DescribeClusters"
            | "DeleteCluster"
            | "CreateClusterSnapshot"
            | "DescribeClusterSnapshots"
            | "DeleteClusterSnapshot"
            | "ModifyCluster"
            | "RebootCluster"
            | "CreateClusterSubnetGroup"
            | "DescribeClusterSubnetGroups"
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
