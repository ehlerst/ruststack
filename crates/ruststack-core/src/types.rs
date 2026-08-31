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
