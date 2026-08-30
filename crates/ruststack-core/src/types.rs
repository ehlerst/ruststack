use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AwsService {
    S3,
    Sqs,
    Sns,
    EventBridge,
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
