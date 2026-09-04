use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Destination {
    #[serde(default)]
    pub to_addresses: Vec<String>,
    #[serde(default)]
    pub cc_addresses: Vec<String>,
    #[serde(default)]
    pub bcc_addresses: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Content {
    pub data: String,
    pub charset: Option<String>,
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Self {
            data: s.to_string(),
            charset: None,
        }
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Self {
            data: s,
            charset: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BodyContent {
    pub text: Option<Content>,
    pub html: Option<Content>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Message {
    pub subject: Content,
    pub body: BodyContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MessageTag {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendEmailRequest {
    pub source: String,
    pub destination: Destination,
    pub message: Message,
    pub reply_to_addresses: Option<Vec<String>>,
    pub return_path: Option<String>,
    pub source_arn: Option<String>,
    pub return_path_arn: Option<String>,
    pub tags: Option<Vec<MessageTag>>,
    pub configuration_set_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendEmailResponse {
    pub message_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawMessage {
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendRawEmailRequest {
    pub source: Option<String>,
    pub destinations: Option<Vec<String>>,
    pub raw_message: RawMessage,
    pub from_arn: Option<String>,
    pub source_arn: Option<String>,
    pub return_path_arn: Option<String>,
    pub tags: Option<Vec<MessageTag>>,
    pub configuration_set_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendRawEmailResponse {
    pub message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VerifyEmailIdentityRequest {
    pub email_address: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VerifyEmailIdentityResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VerifyDomainIdentityRequest {
    pub domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VerifyDomainIdentityResponse {
    pub verification_token: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListIdentitiesRequest {
    pub identity_type: Option<String>,
    pub max_items: Option<usize>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListIdentitiesResponse {
    pub identities: Vec<String>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteIdentityRequest {
    pub identity: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteIdentityResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetSendQuotaResponse {
    pub max_24_hour_send: f64,
    pub max_send_rate: f64,
    pub sent_last_24_hours: f64,
}

impl Default for GetSendQuotaResponse {
    fn default() -> Self {
        Self {
            max_24_hour_send: 200.0,
            max_send_rate: 1.0,
            sent_last_24_hours: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendDataPoint {
    pub timestamp: String,
    pub delivery_attempts: i64,
    pub bounces: i64,
    pub complaints: i64,
    pub rejects: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetSendStatisticsResponse {
    pub send_data_points: Vec<SendDataPoint>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetIdentityVerificationAttributesRequest {
    pub identities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IdentityVerificationAttributes {
    pub verification_status: String,
    pub verification_token: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetIdentityVerificationAttributesResponse {
    pub verification_attributes: HashMap<String, IdentityVerificationAttributes>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SentEmail {
    pub timestamp: String,
    pub message_id: String,
    pub source: String,
    pub destinations: Vec<String>,
    pub subject: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub raw_data: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Template {
    pub template_name: String,
    pub subject_part: Option<String>,
    pub text_part: Option<String>,
    pub html_part: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateTemplateRequest {
    pub template: Template,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetTemplateRequest {
    pub template_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetTemplateResponse {
    pub template: Template,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TemplateMetadata {
    pub name: String,
    pub created_timestamp: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListTemplatesResponse {
    pub templates_metadata: Vec<TemplateMetadata>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UpdateTemplateRequest {
    pub template: Template,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteTemplateRequest {
    pub template_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendTemplatedEmailRequest {
    pub source: String,
    pub destination: Destination,
    pub template: String,
    pub template_data: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SesStateSnapshot {
    pub identities: Vec<String>,
    pub sent_emails: Vec<SentEmail>,
    #[serde(default)]
    pub templates: Vec<Template>,
}
