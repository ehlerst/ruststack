use axum::http::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum RustStackError {
    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Internal Server Error: {0}")]
    Internal(String),

    #[error("S3 Error [{code}]: {message}")]
    S3 {
        code: String,
        message: String,
        status: StatusCode,
        resource: Option<String>,
    },

    #[error("SQS Error [{code}]: {message}")]
    Sqs {
        code: String,
        message: String,
        status: StatusCode,
        error_type: String, // "Sender" or "Receiver"
    },

    #[error("SNS Error [{code}]: {message}")]
    Sns {
        code: String,
        message: String,
        status: StatusCode,
    },

    #[error("EventBridge Error [{code}]: {message}")]
    EventBridge {
        code: String,
        message: String,
        status: StatusCode,
    },

    #[error("SSM Error [{code}]: {message}")]
    Ssm {
        code: String,
        message: String,
        status: StatusCode,
    },

    #[error("SecretsManager Error [{code}]: {message}")]
    SecretsManager {
        code: String,
        message: String,
        status: StatusCode,
    },

    #[error("STS Error [{code}]: {message}")]
    Sts {
        code: String,
        message: String,
        status: StatusCode,
    },

    #[error("DynamoDB Error [{code}]: {message}")]
    DynamoDb {
        code: String,
        message: String,
        status: StatusCode,
    },
}

impl RustStackError {
    pub fn s3_not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::S3 {
            code: code.into(),
            message: message.into(),
            status: StatusCode::NOT_FOUND,
            resource: None,
        }
    }

    pub fn s3_bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::S3 {
            code: code.into(),
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
            resource: None,
        }
    }

    pub fn sqs_bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Sqs {
            code: code.into(),
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
            error_type: "Sender".to_string(),
        }
    }

    pub fn sqs_not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Sqs {
            code: code.into(),
            message: message.into(),
            status: StatusCode::NOT_FOUND,
            error_type: "Sender".to_string(),
        }
    }

    pub fn sns_bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Sns {
            code: code.into(),
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn sns_not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Sns {
            code: code.into(),
            message: message.into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    pub fn eventbridge_bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::EventBridge {
            code: code.into(),
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn eventbridge_not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::EventBridge {
            code: code.into(),
            message: message.into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    pub fn ssm_bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Ssm {
            code: code.into(),
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn ssm_not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Ssm {
            code: code.into(),
            message: message.into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    pub fn secretsmanager_bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::SecretsManager {
            code: code.into(),
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn secretsmanager_not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::SecretsManager {
            code: code.into(),
            message: message.into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    pub fn sts_bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Sts {
            code: code.into(),
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn dynamodb_bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::DynamoDb {
            code: code.into(),
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn dynamodb_not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::DynamoDb {
            code: code.into(),
            message: message.into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    pub fn to_s3_xml(&self, request_id: &str) -> String {
        let (code, message, resource) = match self {
            Self::S3 {
                code,
                message,
                resource,
                ..
            } => (code.as_str(), message.as_str(), resource.as_deref()),
            Self::NotFound(msg) => ("NoSuchKey", msg.as_str(), None),
            Self::BadRequest(msg) => ("InvalidArgument", msg.as_str(), None),
            Self::Internal(msg) => ("InternalError", msg.as_str(), None),
            _ => ("InternalError", "An internal error occurred.", None),
        };

        let resource_tag = match resource {
            Some(res) => format!("<Resource>{}</Resource>", quick_xml::escape::escape(res)),
            None => String::new(),
        };

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>{}</Code>
    <Message>{}</Message>
    {}
    <RequestId>{}</RequestId>
    <HostId>ruststack-host-id</HostId>
</Error>"#,
            quick_xml::escape::escape(code),
            quick_xml::escape::escape(message),
            resource_tag,
            quick_xml::escape::escape(request_id)
        )
    }

    pub fn to_sqs_xml(&self, request_id: &str) -> String {
        let (code, message, error_type) = match self {
            Self::Sqs {
                code,
                message,
                error_type,
                ..
            } => (code.as_str(), message.as_str(), error_type.as_str()),
            Self::NotFound(msg) => ("QueueDoesNotExist", msg.as_str(), "Sender"),
            Self::BadRequest(msg) => ("InvalidParameterValue", msg.as_str(), "Sender"),
            _ => (
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist.",
                "Sender",
            ),
        };

        format!(
            r#"<?xml version="1.0"?>
<ErrorResponse xmlns="http://queue.amazonaws.com/doc/2012-11-05/">
    <Error>
        <Type>{}</Type>
        <Code>{}</Code>
        <Message>{}</Message>
        <Detail/>
    </Error>
    <RequestId>{}</RequestId>
</ErrorResponse>"#,
            quick_xml::escape::escape(error_type),
            quick_xml::escape::escape(code),
            quick_xml::escape::escape(message),
            quick_xml::escape::escape(request_id)
        )
    }

    pub fn to_sqs_json(&self) -> serde_json::Value {
        let (code, message) = match self {
            Self::Sqs { code, message, .. } => (code.as_str(), message.as_str()),
            Self::NotFound(msg) => ("QueueDoesNotExist", msg.as_str()),
            Self::BadRequest(msg) => ("InvalidParameterValue", msg.as_str()),
            _ => (
                "AWS.SimpleQueueService.NonExistentQueue",
                "The specified queue does not exist.",
            ),
        };

        serde_json::json!({
            "__type": code,
            "message": message
        })
    }

    pub fn to_sns_xml(&self, request_id: &str) -> String {
        let (code, message) = match self {
            Self::Sns { code, message, .. } => (code.as_str(), message.as_str()),
            Self::NotFound(msg) => ("NotFound", msg.as_str()),
            Self::BadRequest(msg) => ("InvalidParameter", msg.as_str()),
            _ => ("InternalError", "An internal error occurred."),
        };

        format!(
            r#"<?xml version="1.0"?>
<ErrorResponse xmlns="http://sns.amazonaws.com/doc/2010-03-31/">
    <Error>
        <Type>Sender</Type>
        <Code>{}</Code>
        <Message>{}</Message>
    </Error>
    <RequestId>{}</RequestId>
</ErrorResponse>"#,
            quick_xml::escape::escape(code),
            quick_xml::escape::escape(message),
            quick_xml::escape::escape(request_id)
        )
    }

    pub fn to_sns_json(&self) -> serde_json::Value {
        let (code, message) = match self {
            Self::Sns { code, message, .. } => (code.as_str(), message.as_str()),
            Self::NotFound(msg) => ("NotFound", msg.as_str()),
            Self::BadRequest(msg) => ("InvalidParameter", msg.as_str()),
            _ => ("InternalError", "An internal error occurred."),
        };

        serde_json::json!({
            "__type": code,
            "message": message
        })
    }

    pub fn to_eventbridge_json(&self) -> serde_json::Value {
        let (code, message) = match self {
            Self::EventBridge { code, message, .. } => (code.as_str(), message.as_str()),
            Self::NotFound(msg) => ("ResourceNotFoundException", msg.as_str()),
            Self::BadRequest(msg) => ("InvalidParameterValueException", msg.as_str()),
            _ => ("InternalException", "An internal error occurred."),
        };

        serde_json::json!({
            "__type": code,
            "message": message
        })
    }

    pub fn to_ssm_json(&self) -> serde_json::Value {
        let (code, message) = match self {
            Self::Ssm { code, message, .. } => (code.as_str(), message.as_str()),
            Self::NotFound(msg) => ("ParameterNotFound", msg.as_str()),
            Self::BadRequest(msg) => ("ValidationException", msg.as_str()),
            _ => ("InternalServerError", "An internal error occurred."),
        };

        serde_json::json!({
            "__type": code,
            "message": message
        })
    }

    pub fn to_secretsmanager_json(&self) -> serde_json::Value {
        let (code, message) = match self {
            Self::SecretsManager { code, message, .. } => (code.as_str(), message.as_str()),
            Self::NotFound(msg) => ("ResourceNotFoundException", msg.as_str()),
            Self::BadRequest(msg) => ("InvalidParameterException", msg.as_str()),
            _ => ("InternalServiceError", "An internal error occurred."),
        };

        serde_json::json!({
            "__type": code,
            "message": message
        })
    }

    pub fn to_sts_xml(&self, request_id: &str) -> String {
        let (code, message) = match self {
            Self::Sts { code, message, .. } => (code.as_str(), message.as_str()),
            Self::NotFound(msg) => ("NoSuchEntity", msg.as_str()),
            Self::BadRequest(msg) => ("ValidationError", msg.as_str()),
            _ => ("InternalFailure", "An internal error occurred."),
        };

        format!(
            r#"<ErrorResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
    <Error>
        <Type>Sender</Type>
        <Code>{}</Code>
        <Message>{}</Message>
    </Error>
    <RequestId>{}</RequestId>
</ErrorResponse>"#,
            quick_xml::escape::escape(code),
            quick_xml::escape::escape(message),
            quick_xml::escape::escape(request_id)
        )
    }

    pub fn to_sts_json(&self) -> serde_json::Value {
        let (code, message) = match self {
            Self::Sts { code, message, .. } => (code.as_str(), message.as_str()),
            Self::NotFound(msg) => ("NoSuchEntity", msg.as_str()),
            Self::BadRequest(msg) => ("ValidationError", msg.as_str()),
            _ => ("InternalFailure", "An internal error occurred."),
        };

        serde_json::json!({
            "__type": code,
            "message": message
        })
    }

    pub fn to_dynamodb_json(&self) -> serde_json::Value {
        let (code, message) = match self {
            Self::DynamoDb { code, message, .. } => (code.as_str(), message.as_str()),
            Self::NotFound(msg) => ("ResourceNotFoundException", msg.as_str()),
            Self::BadRequest(msg) => ("ValidationException", msg.as_str()),
            _ => ("InternalServerError", "An internal error occurred."),
        };

        serde_json::json!({
            "__type": format!("com.amazonaws.dynamodb.v20120810#{}", code),
            "message": message
        })
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::S3 { status, .. } => *status,
            Self::Sqs { status, .. } => *status,
            Self::Sns { status, .. } => *status,
            Self::EventBridge { status, .. } => *status,
            Self::Ssm { status, .. } => *status,
            Self::SecretsManager { status, .. } => *status,
            Self::Sts { status, .. } => *status,
            Self::DynamoDb { status, .. } => *status,
        }
    }
}
