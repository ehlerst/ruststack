use crate::types::*;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum BedrockError {
    #[error("ResourceNotFoundException: Model {0} not found")]
    ModelNotFound(String),
    #[error("ValidationException: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockStateSnapshot {
    pub custom_models: Vec<FoundationModelSummary>,
}

#[derive(Clone)]
pub struct BedrockState {
    account_id: String,
    region: String,
    models: Arc<DashMap<String, FoundationModelSummary>>,
}

impl BedrockState {
    pub fn new(account_id: String, region: String) -> Self {
        let state = Self {
            account_id,
            region,
            models: Arc::new(DashMap::new()),
        };

        state.init_foundation_models();
        state
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    fn init_foundation_models(&self) {
        let default_models = vec![
            (
                "anthropic.claude-3-5-sonnet-20240620-v1:0",
                "Claude 3.5 Sonnet",
                "Anthropic",
                vec!["TEXT", "IMAGE"],
                vec!["TEXT"],
            ),
            (
                "anthropic.claude-3-haiku-20240307-v1:0",
                "Claude 3 Haiku",
                "Anthropic",
                vec!["TEXT", "IMAGE"],
                vec!["TEXT"],
            ),
            (
                "meta.llama3-70b-instruct-v1:0",
                "Llama 3 70B Instruct",
                "Meta",
                vec!["TEXT"],
                vec!["TEXT"],
            ),
            (
                "amazon.titan-text-express-v1",
                "Titan Text G1 - Express",
                "Amazon",
                vec!["TEXT"],
                vec!["TEXT"],
            ),
            (
                "cohere.command-r-v1:0",
                "Command R",
                "Cohere",
                vec!["TEXT"],
                vec!["TEXT"],
            ),
        ];

        for (id, name, provider, in_mods, out_mods) in default_models {
            let arn = format!("arn:aws:bedrock:{}::foundation-model/{}", self.region, id);
            self.models.insert(
                id.to_string(),
                FoundationModelSummary {
                    model_arn: arn,
                    model_id: id.to_string(),
                    model_name: name.to_string(),
                    provider_name: provider.to_string(),
                    input_modalities: in_mods.into_iter().map(String::from).collect(),
                    output_modalities: out_mods.into_iter().map(String::from).collect(),
                    response_streaming_supported: true,
                    customizations_supported: vec![],
                    inference_types_supported: vec!["ON_DEMAND".to_string()],
                },
            );
        }
    }

    pub fn list_foundation_models(&self) -> Vec<FoundationModelSummary> {
        self.models.iter().map(|kv| kv.value().clone()).collect()
    }

    pub fn get_foundation_model(&self, model_id: &str) -> Result<FoundationModelSummary, BedrockError> {
        self.models
            .get(model_id)
            .map(|kv| kv.value().clone())
            .ok_or_else(|| BedrockError::ModelNotFound(model_id.to_string()))
    }

    pub fn invoke_model(
        &self,
        model_id: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, BedrockError> {
        // Ensure model exists
        let _model = self.get_foundation_model(model_id)?;

        let request_text = if let Some(prompt) = body.get("prompt").and_then(|p| p.as_str()) {
            prompt.to_string()
        } else if let Some(messages) = body.get("messages").and_then(|m| m.as_array()) {
            messages
                .iter()
                .filter_map(|msg| {
                    if let Some(c) = msg.get("content").and_then(|c| c.as_str()) {
                        Some(c.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            "Simulated response from RustStack Bedrock engine".to_string()
        };

        if model_id.starts_with("anthropic.claude") {
            let prompt_len = request_text.len().max(1);
            let resp = ClaudeResponse {
                id: format!("msg_{}", Uuid::new_v4().to_string().replace('-', "")),
                response_type: "message".to_string(),
                role: "assistant".to_string(),
                content: vec![ClaudeContentBlock {
                    block_type: "text".to_string(),
                    text: format!("RustStack Bedrock Response: Processed input [{}] successfully.", request_text),
                }],
                model: model_id.to_string(),
                stop_reason: "end_turn".to_string(),
                usage: ClaudeUsage {
                    input_tokens: (prompt_len / 4).max(1) as i32,
                    output_tokens: 32,
                },
            };
            Ok(serde_json::to_value(resp).unwrap())
        } else if model_id.starts_with("meta.llama") {
            let resp = serde_json::json!({
                "generation": format!("RustStack Llama Response: Processed input [{}] successfully.", request_text),
                "prompt_token_count": 15,
                "generation_token_count": 25,
                "stop_reason": "stop"
            });
            Ok(resp)
        } else {
            let resp = serde_json::json!({
                "inputTextTokenCount": 10,
                "results": [{
                    "tokenCount": 20,
                    "outputText": format!("RustStack Bedrock Titan Response: Processed input [{}] successfully.", request_text),
                    "completionReason": "FINISH"
                }]
            });
            Ok(resp)
        }
    }

    pub fn reset(&self) {
        self.models.clear();
        self.init_foundation_models();
    }

    pub fn export_snapshot(&self) -> BedrockStateSnapshot {
        BedrockStateSnapshot {
            custom_models: self.models.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: BedrockStateSnapshot) {
        self.models.clear();
        for m in snapshot.custom_models {
            self.models.insert(m.model_id.clone(), m);
        }
    }
}
