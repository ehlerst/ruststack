use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResourceRecord {
    pub name: String,
    pub r#type: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DomainValidation {
    pub domain_name: String,
    pub validation_emails: Vec<String>,
    pub validation_method: String,
    pub validation_status: String,
    pub resource_record: Option<ResourceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CertificateDetail {
    pub certificate_arn: String,
    pub domain_name: String,
    pub subject_alternative_names: Vec<String>,
    pub domain_validation_options: Vec<DomainValidation>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub issued_at: Option<DateTime<Utc>>,
    pub not_before: Option<DateTime<Utc>>,
    pub not_after: Option<DateTime<Utc>>,
    pub key_algorithm: String,
    pub signature_algorithm: String,
    pub r#type: String,
    pub in_use_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CertificateSummary {
    pub certificate_arn: String,
    pub domain_name: String,
    pub subject_alternative_names: Vec<String>,
    pub status: String,
    pub r#type: String,
    pub key_algorithm: String,
    pub created_at: DateTime<Utc>,
    pub in_use_by: Vec<String>,
}
