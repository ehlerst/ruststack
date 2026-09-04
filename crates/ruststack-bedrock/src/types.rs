use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundationModelSummary {
    pub model_arn: String,
    pub model_id: String,
    pub model_name: String,
    pub provider_name: String,
    pub input_modalities: Vec<String>,
    pub output_modalities: Vec<String>,
    pub response_streaming_supported: bool,
    pub customizations_supported: Vec<String>,
    pub inference_types_supported: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFoundationModelsResponse {
    pub model_summaries: Vec<FoundationModelSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFoundationModelResponse {
    pub model_details: FoundationModelSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeRequest {
    pub anthropic_version: Option<String>,
    pub max_tokens: Option<i32>,
    pub messages: Option<Vec<ClaudeMessage>>,
    pub prompt: Option<String>,
    pub system: Option<String>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ClaudeContentBlock>,
    pub model: String,
    pub stop_reason: String,
    pub usage: ClaudeUsage,
}
