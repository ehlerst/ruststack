use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AwsService {
    S3,
    Sqs,
    Sns,
    EventBridge,
    Ssm,
    SecretsManager,
    Sts,
    DynamoDb,
    Kms,
    Logs,
    Iam,
    Ses,
    Kinesis,
    Lambda,
    CloudWatch,
    DynamoDbStreams,
    Cognito,
    ApiGateway,
    Route53,
    StepFunctions,
    CloudFormation,
    Ecr,
    Ecs,
    Ec2,
    Elbv2,
    Bedrock,
    OpenSearch,
    Athena,
    Rds,
    ElastiCache,
    Redshift,
    Acm,
    WafV2,
    Organizations,
    Internal,
    Unknown,
}

impl fmt::Display for AwsService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AwsService::S3 => write!(f, "s3"),
            AwsService::Sqs => write!(f, "sqs"),
            AwsService::Sns => write!(f, "sns"),
            AwsService::EventBridge => write!(f, "events"),
            AwsService::Ssm => write!(f, "ssm"),
            AwsService::SecretsManager => write!(f, "secretsmanager"),
            AwsService::Sts => write!(f, "sts"),
            AwsService::DynamoDb => write!(f, "dynamodb"),
            AwsService::Kms => write!(f, "kms"),
            AwsService::Logs => write!(f, "logs"),
            AwsService::Iam => write!(f, "iam"),
            AwsService::Ses => write!(f, "ses"),
            AwsService::Kinesis => write!(f, "kinesis"),
            AwsService::Lambda => write!(f, "lambda"),
            AwsService::CloudWatch => write!(f, "cloudwatch"),
            AwsService::DynamoDbStreams => write!(f, "dynamodbstreams"),
            AwsService::Cognito => write!(f, "cognito"),
            AwsService::ApiGateway => write!(f, "apigateway"),
            AwsService::Route53 => write!(f, "route53"),
            AwsService::StepFunctions => write!(f, "states"),
            AwsService::CloudFormation => write!(f, "cloudformation"),
            AwsService::Ecr => write!(f, "ecr"),
            AwsService::Ecs => write!(f, "ecs"),
            AwsService::Ec2 => write!(f, "ec2"),
            AwsService::Elbv2 => write!(f, "elasticloadbalancing"),
            AwsService::Bedrock => write!(f, "bedrock"),
            AwsService::OpenSearch => write!(f, "opensearch"),
            AwsService::Athena => write!(f, "athena"),
            AwsService::Rds => write!(f, "rds"),
            AwsService::ElastiCache => write!(f, "elasticache"),
            AwsService::Redshift => write!(f, "redshift"),
            AwsService::Acm => write!(f, "acm"),
            AwsService::WafV2 => write!(f, "wafv2"),
            AwsService::Organizations => write!(f, "organizations"),
            AwsService::Internal => write!(f, "internal"),
            AwsService::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestMetadata {
    pub service: AwsService,
    pub region: String,
    pub account_id: String,
    pub request_id: String,
}

impl Default for RequestMetadata {
    fn default() -> Self {
        Self {
            service: AwsService::Unknown,
            region: "us-east-1".to_string(),
            account_id: "000000000000".to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}
