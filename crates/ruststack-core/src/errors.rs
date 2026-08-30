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

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::S3 { status, .. } => *status,
            Self::Sqs { status, .. } => *status,
        }
    }
}
