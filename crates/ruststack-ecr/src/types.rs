use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub repository_arn: String,
    pub registry_id: String,
    pub repository_name: String,
    pub repository_uri: String,
    pub created_at: f64,
    pub image_tag_mutability: Option<String>,
    pub image_scanning_configuration: Option<serde_json::Value>,
    pub encryption_configuration: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageIdentifier {
    pub image_digest: Option<String>,
    pub image_tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub registry_id: String,
    pub repository_name: String,
    pub image_id: ImageIdentifier,
    pub image_manifest: Option<String>,
    pub image_manifest_media_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDetail {
    pub registry_id: String,
    pub repository_name: String,
    pub image_digest: String,
    pub image_tags: Vec<String>,
    pub image_size_in_bytes: i64,
    pub image_pushed_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationData {
    pub authorization_token: String,
    pub expires_at: f64,
    pub proxy_endpoint: String,
}

// ----------------------------------------------------------------------------
// Request / Response types
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRepositoryRequest {
    pub repository_name: String,
    pub image_tag_mutability: Option<String>,
    pub tags: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRepositoryResponse {
    pub repository: Repository,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DescribeRepositoriesRequest {
    pub repository_names: Option<Vec<String>>,
    pub registry_id: Option<String>,
    pub max_results: Option<usize>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeRepositoriesResponse {
    pub repositories: Vec<Repository>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRepositoryRequest {
    pub repository_name: String,
    pub force: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRepositoryResponse {
    pub repository: Repository,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetAuthorizationTokenRequest {
    pub registry_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAuthorizationTokenResponse {
    pub authorization_data: Vec<AuthorizationData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutImageRequest {
    pub repository_name: String,
    pub image_manifest: String,
    pub image_tag: Option<String>,
    pub image_digest: Option<String>,
    pub image_manifest_media_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutImageResponse {
    pub image: Image,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchGetImageRequest {
    pub repository_name: String,
    pub image_ids: Vec<ImageIdentifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchGetImageResponse {
    pub images: Vec<Image>,
    pub failures: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListImagesRequest {
    pub repository_name: String,
    pub max_results: Option<usize>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListImagesResponse {
    pub image_ids: Vec<ImageIdentifier>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDeleteImageRequest {
    pub repository_name: String,
    pub image_ids: Vec<ImageIdentifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDeleteImageResponse {
    pub image_ids: Vec<ImageIdentifier>,
    pub failures: Vec<serde_json::Value>,
}

// ----------------------------------------------------------------------------
// State & Snapshot Models
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredImage {
    pub digest: String,
    pub tag: Option<String>,
    pub manifest: String,
    pub media_type: Option<String>,
    pub pushed_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRepository {
    pub repository: Repository,
    pub images: Vec<StoredImage>,
    pub policy_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EcrStateSnapshot {
    pub repositories: HashMap<String, StoredRepository>,
}
