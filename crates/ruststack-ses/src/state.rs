use crate::types::*;
use base64::Engine;
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum SesError {
    #[error("NotFound: {0}")]
    NotFound(String),
    #[error("AlreadyExists: {0}")]
    AlreadyExists(String),
    #[error("ValidationError: {0}")]
    Validation(String),
    #[error("InvalidParameterValue: {0}")]
    InvalidParameter(String),
    #[error("MessageRejected: {0}")]
    MessageRejected(String),
}

#[derive(Clone)]
pub struct SesState {
    pub account_id: String,
    pub region: String,
    identities: Arc<DashMap<String, ()>>,
    templates: Arc<DashMap<String, Template>>,
    sent_emails: Arc<RwLock<Vec<SentEmail>>>,
}

impl Default for SesState {
    fn default() -> Self {
        Self::new("000000000000", "us-east-1")
    }
}

impl SesState {
    pub fn new(account_id: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
            region: region.into(),
            identities: Arc::new(DashMap::new()),
            templates: Arc::new(DashMap::new()),
            sent_emails: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn generate_message_id(&self) -> String {
        format!("010001{:02x}-{}-000000", fastrand::u8(..), Uuid::new_v4())
    }

    pub fn send_email(&self, req: SendEmailRequest) -> Result<SendEmailResponse, SesError> {
        if req.source.trim().is_empty() {
            return Err(SesError::InvalidParameter(
                "Source email address must not be empty".to_string(),
            ));
        }

        let mut destinations = Vec::new();
        destinations.extend(req.destination.to_addresses.clone());
        destinations.extend(req.destination.cc_addresses.clone());
        destinations.extend(req.destination.bcc_addresses.clone());

        if destinations.is_empty() {
            return Err(SesError::InvalidParameter(
                "At least one destination address (To, Cc, or Bcc) must be specified".to_string(),
            ));
        }

        let message_id = self.generate_message_id();
        let now = Utc::now().to_rfc3339();

        let subject = Some(req.message.subject.data.clone());
        let body_text = req.message.body.text.as_ref().map(|t| t.data.clone());
        let body_html = req.message.body.html.as_ref().map(|h| h.data.clone());

        let sent_email = SentEmail {
            timestamp: now,
            message_id: message_id.clone(),
            source: req.source,
            destinations,
            subject,
            body_text,
            body_html,
            raw_data: None,
        };

        self.sent_emails.write().push(sent_email);

        Ok(SendEmailResponse { message_id })
    }

    pub fn send_raw_email(
        &self,
        req: SendRawEmailRequest,
    ) -> Result<SendRawEmailResponse, SesError> {
        if req.raw_message.data.trim().is_empty() {
            return Err(SesError::InvalidParameter(
                "RawMessage.Data must not be empty".to_string(),
            ));
        }

        let message_id = self.generate_message_id();
        let now = Utc::now().to_rfc3339();

        let (
            parsed_source,
            parsed_destinations,
            parsed_subject,
            parsed_body_text,
            parsed_body_html,
        ) = parse_raw_email(&req.raw_message.data);

        let source = req
            .source
            .or(parsed_source)
            .unwrap_or_else(|| "unknown@example.com".to_string());

        let mut destinations = req.destinations.unwrap_or_default();
        if destinations.is_empty() {
            destinations = parsed_destinations;
        }

        let sent_email = SentEmail {
            timestamp: now,
            message_id: message_id.clone(),
            source,
            destinations,
            subject: parsed_subject,
            body_text: parsed_body_text,
            body_html: parsed_body_html,
            raw_data: Some(req.raw_message.data),
        };

        self.sent_emails.write().push(sent_email);

        Ok(SendRawEmailResponse { message_id })
    }

    pub fn verify_email_identity(&self, req: VerifyEmailIdentityRequest) -> Result<(), SesError> {
        if req.email_address.trim().is_empty() {
            return Err(SesError::InvalidParameter(
                "EmailAddress must not be empty".to_string(),
            ));
        }
        self.identities
            .insert(req.email_address.trim().to_string(), ());
        Ok(())
    }

    pub fn verify_domain_identity(
        &self,
        req: VerifyDomainIdentityRequest,
    ) -> Result<VerifyDomainIdentityResponse, SesError> {
        if req.domain.trim().is_empty() {
            return Err(SesError::InvalidParameter(
                "Domain must not be empty".to_string(),
            ));
        }
        let domain = req.domain.trim().to_string();
        let verification_token = Uuid::new_v4().to_string();
        self.identities.insert(domain, ());
        Ok(VerifyDomainIdentityResponse { verification_token })
    }

    pub fn list_identities(
        &self,
        req: ListIdentitiesRequest,
    ) -> Result<ListIdentitiesResponse, SesError> {
        let mut list: Vec<String> = self.identities.iter().map(|kv| kv.key().clone()).collect();

        if let Some(ref itype) = req.identity_type {
            match itype.as_str() {
                "EmailAddress" => {
                    list.retain(|id| id.contains('@'));
                }
                "Domain" => {
                    list.retain(|id| !id.contains('@'));
                }
                _ => {}
            }
        }

        list.sort();

        let max_items = req.max_items.unwrap_or(100);
        let next_token = if list.len() > max_items {
            list.truncate(max_items);
            None
        } else {
            None
        };

        Ok(ListIdentitiesResponse {
            identities: list,
            next_token,
        })
    }

    pub fn delete_identity(&self, req: DeleteIdentityRequest) -> Result<(), SesError> {
        if req.identity.trim().is_empty() {
            return Err(SesError::InvalidParameter(
                "Identity must not be empty".to_string(),
            ));
        }
        self.identities.remove(req.identity.trim());
        Ok(())
    }

    pub fn get_send_quota(&self) -> Result<GetSendQuotaResponse, SesError> {
        let sent_count = self.sent_emails.read().len() as f64;
        Ok(GetSendQuotaResponse {
            max_24_hour_send: 200.0,
            max_send_rate: 1.0,
            sent_last_24_hours: sent_count,
        })
    }

    pub fn get_send_statistics(&self) -> Result<GetSendStatisticsResponse, SesError> {
        let sent_count = self.sent_emails.read().len() as i64;
        let point = SendDataPoint {
            timestamp: Utc::now().to_rfc3339(),
            delivery_attempts: sent_count,
            bounces: 0,
            complaints: 0,
            rejects: 0,
        };
        Ok(GetSendStatisticsResponse {
            send_data_points: vec![point],
        })
    }

    pub fn get_identity_verification_attributes(
        &self,
        req: GetIdentityVerificationAttributesRequest,
    ) -> Result<GetIdentityVerificationAttributesResponse, SesError> {
        let mut map = HashMap::new();
        for id in req.identities {
            if self.identities.contains_key(&id) {
                map.insert(
                    id,
                    IdentityVerificationAttributes {
                        verification_status: "Success".to_string(),
                        verification_token: None,
                    },
                );
            }
        }
        Ok(GetIdentityVerificationAttributesResponse {
            verification_attributes: map,
        })
    }

    pub fn create_template(&self, req: CreateTemplateRequest) -> Result<(), SesError> {
        let name = req.template.template_name.trim();
        if name.is_empty() {
            return Err(SesError::InvalidParameter("TemplateName is required".to_string()));
        }
        if self.templates.contains_key(name) {
            return Err(SesError::AlreadyExists(format!("Template {} already exists", name)));
        }
        self.templates.insert(name.to_string(), req.template);
        Ok(())
    }

    pub fn get_template(&self, template_name: &str) -> Result<Template, SesError> {
        self.templates
            .get(template_name)
            .map(|t| t.clone())
            .ok_or_else(|| SesError::NotFound(format!("Template {} does not exist", template_name)))
    }

    pub fn list_templates(&self) -> Result<Vec<TemplateMetadata>, SesError> {
        let mut list: Vec<TemplateMetadata> = self
            .templates
            .iter()
            .map(|item| TemplateMetadata {
                name: item.key().clone(),
                created_timestamp: Some(Utc::now().to_rfc3339()),
            })
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }

    pub fn update_template(&self, req: UpdateTemplateRequest) -> Result<(), SesError> {
        let name = req.template.template_name.trim();
        if !self.templates.contains_key(name) {
            return Err(SesError::NotFound(format!("Template {} does not exist", name)));
        }
        self.templates.insert(name.to_string(), req.template);
        Ok(())
    }

    pub fn delete_template(&self, template_name: &str) -> Result<(), SesError> {
        self.templates.remove(template_name);
        Ok(())
    }

    pub fn send_templated_email(
        &self,
        req: SendTemplatedEmailRequest,
    ) -> Result<SendEmailResponse, SesError> {
        if req.source.trim().is_empty() {
            return Err(SesError::InvalidParameter(
                "Source email address must not be empty".to_string(),
            ));
        }

        let mut destinations = Vec::new();
        destinations.extend(req.destination.to_addresses.clone());
        destinations.extend(req.destination.cc_addresses.clone());
        destinations.extend(req.destination.bcc_addresses.clone());

        if destinations.is_empty() {
            return Err(SesError::InvalidParameter(
                "At least one destination address (To, Cc, or Bcc) must be specified".to_string(),
            ));
        }

        let template = self.get_template(&req.template)?;
        let template_data_val: serde_json::Value =
            serde_json::from_str(&req.template_data).unwrap_or(serde_json::json!({}));

        let subject = template
            .subject_part
            .as_deref()
            .map(|s| render_template(s, &template_data_val));
        let body_text = template
            .text_part
            .as_deref()
            .map(|t| render_template(t, &template_data_val));
        let body_html = template
            .html_part
            .as_deref()
            .map(|h| render_template(h, &template_data_val));

        let message_id = self.generate_message_id();
        let now = Utc::now().to_rfc3339();

        let sent_email = SentEmail {
            timestamp: now,
            message_id: message_id.clone(),
            source: req.source,
            destinations,
            subject,
            body_text,
            body_html,
            raw_data: None,
        };

        self.sent_emails.write().push(sent_email);

        Ok(SendEmailResponse { message_id })
    }

    pub fn get_sent_emails(&self) -> Vec<SentEmail> {
        self.sent_emails.read().clone()
    }

    pub fn clear(&self) {
        self.identities.clear();
        self.templates.clear();
        self.sent_emails.write().clear();
    }

    pub fn export_snapshot(&self) -> SesStateSnapshot {
        SesStateSnapshot {
            identities: self.identities.iter().map(|kv| kv.key().clone()).collect(),
            sent_emails: self.sent_emails.read().clone(),
            templates: self.templates.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: SesStateSnapshot) {
        self.identities.clear();
        self.templates.clear();
        for id in snapshot.identities {
            self.identities.insert(id, ());
        }
        for t in snapshot.templates {
            self.templates.insert(t.template_name.clone(), t);
        }
        let mut emails = self.sent_emails.write();
        emails.clear();
        emails.extend(snapshot.sent_emails);
    }

    pub fn reset(&self) {
        self.clear();
    }
}

fn render_template(template_str: &str, data: &serde_json::Value) -> String {
    let mut rendered = template_str.to_string();
    if let Some(obj) = data.as_object() {
        for (k, v) in obj {
            let placeholder = format!("{{{{{}}}}}", k);
            let val_str = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            rendered = rendered.replace(&placeholder, &val_str);
        }
    }
    rendered
}

fn parse_raw_email(
    raw_input: &str,
) -> (
    Option<String>,
    Vec<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let raw_bytes =
        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(raw_input.trim()) {
            decoded
        } else {
            raw_input.as_bytes().to_vec()
        };
    let content = String::from_utf8_lossy(&raw_bytes).to_string();

    let mut source = None;
    let mut destinations = Vec::new();
    let mut subject = None;
    let mut body_text = None;
    let mut body_html = None;

    let parts: Vec<&str> = content.splitn(2, "\r\n\r\n").collect();
    let (header_part, body_part) = if parts.len() == 2 {
        (parts[0], Some(parts[1]))
    } else {
        let parts_lf: Vec<&str> = content.splitn(2, "\n\n").collect();
        if parts_lf.len() == 2 {
            (parts_lf[0], Some(parts_lf[1]))
        } else {
            (content.as_str(), None)
        }
    };

    for line in header_part.lines() {
        let line = line.trim();
        if let Some(stripped) = line.strip_prefix("From:") {
            source = Some(
                stripped
                    .trim()
                    .trim_matches(|c| c == '<' || c == '>')
                    .to_string(),
            );
        } else if let Some(stripped) = line.strip_prefix("To:") {
            for addr in stripped.split(',') {
                let clean = addr.trim().trim_matches(|c| c == '<' || c == '>');
                if !clean.is_empty() {
                    destinations.push(clean.to_string());
                }
            }
        } else if let Some(stripped) = line.strip_prefix("Subject:") {
            subject = Some(stripped.trim().to_string());
        }
    }

    if let Some(body) = body_part {
        if body.contains("<html") || body.contains("<HTML") || body.contains("<!DOCTYPE") {
            body_html = Some(body.to_string());
        } else {
            body_text = Some(body.to_string());
        }
    }

    (source, destinations, subject, body_text, body_html)
}
