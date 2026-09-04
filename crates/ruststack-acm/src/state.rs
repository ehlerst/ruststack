use crate::types::*;
use chrono::{Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum AcmError {
    #[error("ResourceNotFoundException: Certificate {0} not found")]
    ResourceNotFound(String),
    #[error("InvalidArnException: Invalid certificate ARN {0}")]
    InvalidArn(String),
    #[error("InvalidParameterException: {0}")]
    InvalidParameter(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcmStateSnapshot {
    pub certificates: Vec<CertificateDetail>,
}

#[derive(Clone)]
pub struct AcmState {
    account_id: String,
    region: String,
    certificates: Arc<DashMap<String, CertificateDetail>>,
}

impl AcmState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            certificates: Arc::new(DashMap::new()),
        }
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn request_certificate(
        &self,
        domain_name: String,
        subject_alternative_names: Option<Vec<String>>,
        validation_method: Option<String>,
        key_algorithm: Option<String>,
    ) -> Result<String, AcmError> {
        let cert_id = Uuid::new_v4().to_string();
        let arn = format!(
            "arn:aws:acm:{}:{}:certificate/{}",
            self.region, self.account_id, cert_id
        );

        let now = Utc::now();
        let val_method = validation_method.unwrap_or_else(|| "DNS".to_string());

        let mut sans = subject_alternative_names.unwrap_or_default();
        if !sans.contains(&domain_name) {
            sans.insert(0, domain_name.clone());
        }

        let mut domain_validations = Vec::new();
        for san in &sans {
            let record_hash = format!("{:.16}", Uuid::new_v4().simple());
            let cname_name = format!("_{}.{}", record_hash, san);
            let cname_val = format!("_{}.acm-validations.aws.", &record_hash);

            domain_validations.push(DomainValidation {
                domain_name: san.clone(),
                validation_emails: vec![],
                validation_method: val_method.clone(),
                validation_status: "SUCCESS".to_string(),
                resource_record: Some(ResourceRecord {
                    name: cname_name,
                    r#type: "CNAME".to_string(),
                    value: cname_val,
                }),
            });
        }

        let detail = CertificateDetail {
            certificate_arn: arn.clone(),
            domain_name,
            subject_alternative_names: sans,
            domain_validation_options: domain_validations,
            status: "ISSUED".to_string(),
            created_at: now,
            issued_at: Some(now),
            not_before: Some(now),
            not_after: Some(now + Duration::days(365)),
            key_algorithm: key_algorithm.unwrap_or_else(|| "RSA_2048".to_string()),
            signature_algorithm: "SHA256WITHRSA".to_string(),
            r#type: "AMAZON_ISSUED".to_string(),
            in_use_by: vec![],
        };

        self.certificates.insert(arn.clone(), detail);
        Ok(arn)
    }

    pub fn describe_certificate(&self, cert_arn: &str) -> Result<CertificateDetail, AcmError> {
        self.certificates
            .get(cert_arn)
            .map(|kv| kv.value().clone())
            .ok_or_else(|| AcmError::ResourceNotFound(cert_arn.to_string()))
    }

    pub fn list_certificates(&self) -> Vec<CertificateSummary> {
        self.certificates
            .iter()
            .map(|kv| {
                let c = kv.value();
                CertificateSummary {
                    certificate_arn: c.certificate_arn.clone(),
                    domain_name: c.domain_name.clone(),
                    subject_alternative_names: c.subject_alternative_names.clone(),
                    status: c.status.clone(),
                    r#type: c.r#type.clone(),
                    key_algorithm: c.key_algorithm.clone(),
                    created_at: c.created_at,
                    in_use_by: c.in_use_by.clone(),
                }
            })
            .collect()
    }

    pub fn delete_certificate(&self, cert_arn: &str) -> Result<(), AcmError> {
        self.certificates
            .remove(cert_arn)
            .map(|_| ())
            .ok_or_else(|| AcmError::ResourceNotFound(cert_arn.to_string()))
    }

    pub fn get_certificate(&self, cert_arn: &str) -> Result<(String, String), AcmError> {
        let cert = self.describe_certificate(cert_arn)?;
        let mock_cert = format!(
            "-----BEGIN CERTIFICATE-----\nMIIBkTCB+wIBADANBgkqhkiG9w0BAQsFADARMQ8wDQYDVQQDEwZhbWF6b24wHhcN\n-----END CERTIFICATE-----\n# Domain: {}",
            cert.domain_name
        );
        let mock_chain = "-----BEGIN CERTIFICATE-----\nMIIBkTCB+wIBADANBgkqhkiG9w0BAQsFADARMQ8wDQYDVQQDEwZhbWF6b24wHhcN\n-----END CERTIFICATE-----".to_string();
        Ok((mock_cert, mock_chain))
    }

    pub fn reset(&self) {
        self.certificates.clear();
    }

    pub fn export_snapshot(&self) -> AcmStateSnapshot {
        AcmStateSnapshot {
            certificates: self.certificates.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: AcmStateSnapshot) {
        self.certificates.clear();
        for cert in snapshot.certificates {
            self.certificates.insert(cert.certificate_arn.clone(), cert);
        }
    }
}
