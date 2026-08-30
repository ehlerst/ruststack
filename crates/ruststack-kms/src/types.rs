use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct KeyMetadata {
    #[serde(rename = "AWSAccountId")]
    pub aws_account_id: String,
    pub key_id: String,
    pub arn: String,
    pub creation_date: f64,
    pub enabled: bool,
    pub description: String,
    pub key_usage: String,
    pub key_state: String,
    pub key_spec: String,
    pub origin: String,
    pub key_manager: String,
    #[serde(rename = "CustomerMasterKeySpec")]
    pub customer_master_key_spec: String,
    pub encryption_algorithms: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletion_date: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct KmsKeyEntry {
    pub metadata: KeyMetadata,
    #[serde(default)]
    pub key_bytes: Vec<u8>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AliasEntry {
    pub alias_name: String,
    pub alias_arn: String,
    pub target_key_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated_date: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateKeyRequest {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_key_usage")]
    pub key_usage: String,
    #[serde(default = "default_key_spec")]
    pub key_spec: String,
    #[serde(default = "default_key_spec")]
    pub customer_master_key_spec: String,
    #[serde(default)]
    pub tags: Option<Vec<Tag>>,
}

fn default_key_usage() -> String {
    "ENCRYPT_DECRYPT".to_string()
}

fn default_key_spec() -> String {
    "SYMMETRIC_DEFAULT".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Tag {
    pub tag_key: String,
    pub tag_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeKeyRequest {
    pub key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListKeysRequest {
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub marker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateAliasRequest {
    pub alias_name: String,
    pub target_key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteAliasRequest {
    pub alias_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListAliasesRequest {
    #[serde(default)]
    pub key_id: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub marker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EncryptRequest {
    pub key_id: String,
    pub plaintext: String, // Base64
    #[serde(default = "default_encryption_algorithm")]
    pub encryption_algorithm: String,
    #[serde(default)]
    pub encryption_context: Option<HashMap<String, String>>,
}

fn default_encryption_algorithm() -> String {
    "SYMMETRIC_DEFAULT".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DecryptRequest {
    pub ciphertext_blob: String, // Base64
    #[serde(default)]
    pub key_id: Option<String>,
    #[serde(default = "default_encryption_algorithm")]
    pub encryption_algorithm: String,
    #[serde(default)]
    pub encryption_context: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GenerateDataKeyRequest {
    pub key_id: String,
    #[serde(default = "default_data_key_spec")]
    pub key_spec: String,
    #[serde(default)]
    pub number_of_bytes: Option<usize>,
    #[serde(default)]
    pub encryption_context: Option<HashMap<String, String>>,
}

fn default_data_key_spec() -> String {
    "AES_256".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct KeyIdOnlyRequest {
    pub key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ScheduleKeyDeletionRequest {
    pub key_id: String,
    #[serde(default = "default_pending_window_in_days")]
    pub pending_window_in_days: u32,
}

fn default_pending_window_in_days() -> u32 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KmsStateSnapshot {
    pub keys: Vec<KmsKeyEntry>,
    pub aliases: Vec<AliasEntry>,
}
