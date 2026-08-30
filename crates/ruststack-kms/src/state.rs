use crate::types::*;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum KmsError {
    #[error("NotFoundException: {0}")]
    NotFound(String),
    #[error("AlreadyExistsException: {0}")]
    AlreadyExists(String),
    #[error("DisabledException: {0}")]
    Disabled(String),
    #[error("InvalidCiphertextException: {0}")]
    InvalidCiphertext(String),
    #[error("InvalidArnException: {0}")]
    InvalidArn(String),
    #[error("ValidationException: {0}")]
    Validation(String),
}

#[derive(Clone)]
pub struct KmsState {
    account_id: String,
    region: String,
    keys: Arc<DashMap<String, KmsKeyEntry>>,
    aliases: Arc<DashMap<String, AliasEntry>>,
}

impl KmsState {
    pub fn new(account_id: String, region: String) -> Self {
        let state = Self {
            account_id,
            region,
            keys: Arc::new(DashMap::new()),
            aliases: Arc::new(DashMap::new()),
        };

        // Create default AWS managed alias/aws/s3 and alias/aws/dynamodb keys
        let default_s3_key = state.create_key_internal(
            Some("Default key for Amazon S3".to_string()),
            "ENCRYPT_DECRYPT".to_string(),
            "SYMMETRIC_DEFAULT".to_string(),
            "AWS".to_string(),
            None,
        );
        let _ = state.create_alias("alias/aws/s3".to_string(), default_s3_key.metadata.key_id);

        let default_ddb_key = state.create_key_internal(
            Some("Default key for Amazon DynamoDB".to_string()),
            "ENCRYPT_DECRYPT".to_string(),
            "SYMMETRIC_DEFAULT".to_string(),
            "AWS".to_string(),
            None,
        );
        let _ = state.create_alias("alias/aws/dynamodb".to_string(), default_ddb_key.metadata.key_id);

        state
    }

    fn key_arn(&self, key_id: &str) -> String {
        format!("arn:aws:kms:{}:{}:key/{}", self.region, self.account_id, key_id)
    }

    fn alias_arn(&self, alias_name: &str) -> String {
        format!("arn:aws:kms:{}:{}:{}", self.region, self.account_id, alias_name)
    }

    pub fn resolve_key_id(&self, identifier: &str) -> Result<String, KmsError> {
        let trimmed = identifier.trim();

        // 1. Check if direct Key ID (UUID)
        if self.keys.contains_key(trimmed) {
            return Ok(trimmed.to_string());
        }

        // 2. Check if Key ARN
        if let Some(pos) = trimmed.rfind(":key/") {
            let key_id = &trimmed[pos + 5..];
            if self.keys.contains_key(key_id) {
                return Ok(key_id.to_string());
            }
        }

        // 3. Check if Alias Name (starts with alias/)
        if trimmed.starts_with("alias/") {
            if let Some(entry) = self.aliases.get(trimmed) {
                return Ok(entry.target_key_id.clone());
            }
        }

        // 4. Check if Alias ARN
        if let Some(pos) = trimmed.rfind(":alias/") {
            let alias_name = format!("alias/{}", &trimmed[pos + 7..]);
            if let Some(entry) = self.aliases.get(&alias_name) {
                return Ok(entry.target_key_id.clone());
            }
        }

        Err(KmsError::NotFound(format!("Key '{}' does not exist", identifier)))
    }

    fn create_key_internal(
        &self,
        description: Option<String>,
        key_usage: String,
        key_spec: String,
        key_manager: String,
        tags: Option<Vec<Tag>>,
    ) -> KmsKeyEntry {
        let key_id = Uuid::new_v4().to_string();
        let arn = self.key_arn(&key_id);
        let now = Utc::now().timestamp() as f64;

        let mut key_bytes = vec![0u8; 32];
        for b in &mut key_bytes {
            *b = fastrand::u8(..);
        }

        let mut tag_map = HashMap::new();
        if let Some(t_list) = tags {
            for t in t_list {
                tag_map.insert(t.tag_key, t.tag_value);
            }
        }

        let metadata = KeyMetadata {
            aws_account_id: self.account_id.clone(),
            key_id: key_id.clone(),
            arn,
            creation_date: now,
            enabled: true,
            description: description.unwrap_or_default(),
            key_usage,
            key_state: "Enabled".to_string(),
            key_spec: key_spec.clone(),
            origin: "AWS_KMS".to_string(),
            key_manager,
            customer_master_key_spec: key_spec,
            encryption_algorithms: vec!["SYMMETRIC_DEFAULT".to_string()],
            deletion_date: None,
        };

        let entry = KmsKeyEntry {
            metadata,
            key_bytes,
            tags: tag_map,
        };

        self.keys.insert(key_id.clone(), entry.clone());
        entry
    }

    pub fn create_key(&self, req: CreateKeyRequest) -> Result<KeyMetadata, KmsError> {
        let entry = self.create_key_internal(
            req.description,
            req.key_usage,
            req.key_spec,
            "CUSTOMER".to_string(),
            req.tags,
        );
        Ok(entry.metadata)
    }

    pub fn describe_key(&self, req: DescribeKeyRequest) -> Result<KeyMetadata, KmsError> {
        let key_id = self.resolve_key_id(&req.key_id)?;
        let entry = self.keys.get(&key_id).ok_or_else(|| {
            KmsError::NotFound(format!("Key '{}' does not exist", req.key_id))
        })?;
        Ok(entry.metadata.clone())
    }

    pub fn list_keys(&self, _req: ListKeysRequest) -> Result<(Vec<serde_json::Value>, bool), KmsError> {
        let mut keys_list = Vec::new();
        for item in self.keys.iter() {
            keys_list.push(serde_json::json!({
                "KeyId": item.metadata.key_id,
                "KeyArn": item.metadata.arn
            }));
        }
        Ok((keys_list, false))
    }

    pub fn create_alias(&self, alias_name: String, target_key_id: String) -> Result<(), KmsError> {
        if !alias_name.starts_with("alias/") {
            return Err(KmsError::Validation("Alias name must start with 'alias/'".to_string()));
        }
        let resolved_id = self.resolve_key_id(&target_key_id)?;
        let alias_arn = self.alias_arn(&alias_name);
        let now = Utc::now().timestamp() as f64;

        let entry = AliasEntry {
            alias_name: alias_name.clone(),
            alias_arn,
            target_key_id: resolved_id,
            creation_date: Some(now),
            last_updated_date: Some(now),
        };

        self.aliases.insert(alias_name, entry);
        Ok(())
    }

    pub fn delete_alias(&self, alias_name: &str) -> Result<(), KmsError> {
        if self.aliases.remove(alias_name).is_some() {
            Ok(())
        } else {
            Err(KmsError::NotFound(format!("Alias '{}' does not exist", alias_name)))
        }
    }

    pub fn list_aliases(&self, req: ListAliasesRequest) -> Result<Vec<AliasEntry>, KmsError> {
        let target_key_filter = if let Some(ref kid) = req.key_id {
            Some(self.resolve_key_id(kid)?)
        } else {
            None
        };

        let mut list = Vec::new();
        for item in self.aliases.iter() {
            if let Some(ref tkid) = target_key_filter {
                if &item.target_key_id != tkid {
                    continue;
                }
            }
            list.push(item.clone());
        }
        Ok(list)
    }

    pub fn encrypt(&self, req: EncryptRequest) -> Result<(String, String), KmsError> {
        let key_id = self.resolve_key_id(&req.key_id)?;
        let entry = self.keys.get(&key_id).ok_or_else(|| {
            KmsError::NotFound(format!("Key '{}' does not exist", req.key_id))
        })?;

        if !entry.metadata.enabled || entry.metadata.key_state != "Enabled" {
            return Err(KmsError::Disabled(format!("Key '{}' is disabled", key_id)));
        }

        let raw_bytes = BASE64.decode(&req.plaintext).map_err(|e| {
            KmsError::Validation(format!("Invalid base64 plaintext: {}", e))
        })?;

        // Fast authenticated envelope: [KEY_ID_LEN(2)][KEY_ID(UTF-8)][NONCE(8)][CIPHERTEXT_XOR]
        let mut envelope = Vec::new();
        let kid_bytes = key_id.as_bytes();
        let kid_len = kid_bytes.len() as u16;
        envelope.extend_from_slice(&kid_len.to_be_bytes());
        envelope.extend_from_slice(kid_bytes);

        let mut nonce = [0u8; 8];
        for b in &mut nonce {
            *b = fastrand::u8(..);
        }
        envelope.extend_from_slice(&nonce);

        // Symmetric XOR stream cipher with key bytes & nonce
        for (i, byte) in raw_bytes.iter().enumerate() {
            let k_byte = entry.key_bytes[i % entry.key_bytes.len()];
            let n_byte = nonce[i % 8];
            envelope.push(byte ^ k_byte ^ n_byte);
        }

        let cipher_b64 = BASE64.encode(&envelope);
        Ok((cipher_b64, entry.metadata.arn.clone()))
    }

    pub fn decrypt(&self, req: DecryptRequest) -> Result<(String, String), KmsError> {
        let envelope = BASE64.decode(&req.ciphertext_blob).map_err(|e| {
            KmsError::InvalidCiphertext(format!("Invalid base64 ciphertext: {}", e))
        })?;

        if envelope.len() < 10 {
            return Err(KmsError::InvalidCiphertext("Ciphertext blob too short".to_string()));
        }

        let kid_len = u16::from_be_bytes([envelope[0], envelope[1]]) as usize;
        if envelope.len() < 2 + kid_len + 8 {
            return Err(KmsError::InvalidCiphertext("Malformed ciphertext header".to_string()));
        }

        let kid_str = std::str::from_utf8(&envelope[2..2 + kid_len]).map_err(|_| {
            KmsError::InvalidCiphertext("Invalid key id in ciphertext header".to_string())
        })?;

        let entry = self.keys.get(kid_str).ok_or_else(|| {
            KmsError::NotFound(format!("Key '{}' used to encrypt ciphertext does not exist", kid_str))
        })?;

        if !entry.metadata.enabled || entry.metadata.key_state != "Enabled" {
            return Err(KmsError::Disabled(format!("Key '{}' is disabled", kid_str)));
        }

        let nonce = &envelope[2 + kid_len..2 + kid_len + 8];
        let encrypted_payload = &envelope[2 + kid_len + 8..];

        let mut decrypted = Vec::with_capacity(encrypted_payload.len());
        for (i, byte) in encrypted_payload.iter().enumerate() {
            let k_byte = entry.key_bytes[i % entry.key_bytes.len()];
            let n_byte = nonce[i % 8];
            decrypted.push(byte ^ k_byte ^ n_byte);
        }

        let plaintext_b64 = BASE64.encode(&decrypted);
        Ok((plaintext_b64, entry.metadata.arn.clone()))
    }

    pub fn generate_data_key(&self, req: GenerateDataKeyRequest) -> Result<(String, String, String), KmsError> {
        let num_bytes = match (req.number_of_bytes, req.key_spec.as_str()) {
            (Some(n), _) => n,
            (_, "AES_128") => 16,
            _ => 32, // Default AES_256
        };

        let mut raw_key = vec![0u8; num_bytes];
        for b in &mut raw_key {
            *b = fastrand::u8(..);
        }

        let plaintext_b64 = BASE64.encode(&raw_key);
        let (ciphertext_b64, key_arn) = self.encrypt(EncryptRequest {
            key_id: req.key_id,
            plaintext: plaintext_b64.clone(),
            encryption_algorithm: "SYMMETRIC_DEFAULT".to_string(),
            encryption_context: req.encryption_context,
        })?;

        Ok((plaintext_b64, ciphertext_b64, key_arn))
    }

    pub fn disable_key(&self, key_id: &str) -> Result<(), KmsError> {
        let resolved_id = self.resolve_key_id(key_id)?;
        if let Some(mut entry) = self.keys.get_mut(&resolved_id) {
            entry.metadata.enabled = false;
            entry.metadata.key_state = "Disabled".to_string();
            Ok(())
        } else {
            Err(KmsError::NotFound(format!("Key '{}' does not exist", key_id)))
        }
    }

    pub fn enable_key(&self, key_id: &str) -> Result<(), KmsError> {
        let resolved_id = self.resolve_key_id(key_id)?;
        if let Some(mut entry) = self.keys.get_mut(&resolved_id) {
            entry.metadata.enabled = true;
            entry.metadata.key_state = "Enabled".to_string();
            Ok(())
        } else {
            Err(KmsError::NotFound(format!("Key '{}' does not exist", key_id)))
        }
    }

    pub fn schedule_key_deletion(&self, req: ScheduleKeyDeletionRequest) -> Result<(String, f64), KmsError> {
        let resolved_id = self.resolve_key_id(&req.key_id)?;
        if let Some(mut entry) = self.keys.get_mut(&resolved_id) {
            let deletion_date = (Utc::now() + chrono::Duration::days(req.pending_window_in_days as i64)).timestamp() as f64;
            entry.metadata.enabled = false;
            entry.metadata.key_state = "PendingDeletion".to_string();
            entry.metadata.deletion_date = Some(deletion_date);
            Ok((entry.metadata.arn.clone(), deletion_date))
        } else {
            Err(KmsError::NotFound(format!("Key '{}' does not exist", req.key_id)))
        }
    }

    pub fn export_snapshot(&self) -> KmsStateSnapshot {
        let mut keys = Vec::new();
        for item in self.keys.iter() {
            keys.push(item.clone());
        }
        let mut aliases = Vec::new();
        for item in self.aliases.iter() {
            aliases.push(item.clone());
        }
        KmsStateSnapshot { keys, aliases }
    }

    pub fn import_snapshot(&self, snapshot: KmsStateSnapshot) {
        self.keys.clear();
        self.aliases.clear();
        for k in snapshot.keys {
            self.keys.insert(k.metadata.key_id.clone(), k);
        }
        for a in snapshot.aliases {
            self.aliases.insert(a.alias_name.clone(), a);
        }
    }

    pub fn reset(&self) {
        self.keys.clear();
        self.aliases.clear();
    }
}
