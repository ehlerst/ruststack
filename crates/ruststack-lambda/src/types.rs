use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvocationType {
    #[serde(rename = "RequestResponse")]
    RequestResponse,
    #[serde(rename = "Event")]
    Event,
    #[serde(rename = "DryRun")]
    DryRun,
}

impl Default for InvocationType {
    fn default() -> Self {
        Self::RequestResponse
    }
}

impl fmt::Display for InvocationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequestResponse => write!(f, "RequestResponse"),
            Self::Event => write!(f, "Event"),
            Self::DryRun => write!(f, "DryRun"),
        }
    }
}

impl FromStr for InvocationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "requestresponse" | "request_response" => Ok(Self::RequestResponse),
            "event" => Ok(Self::Event),
            "dryrun" | "dry_run" => Ok(Self::DryRun),
            _ => Err(format!("Invalid invocation type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct FunctionCode {
    #[serde(alias = "zipFile", alias = "zip_file")]
    pub zip_file: Option<String>,
    #[serde(alias = "s3Bucket", alias = "s3_bucket")]
    pub s3_bucket: Option<String>,
    #[serde(alias = "s3Key", alias = "s3_key")]
    pub s3_key: Option<String>,
    #[serde(alias = "s3ObjectVersion", alias = "s3_object_version")]
    pub s3_object_version: Option<String>,
    #[serde(alias = "imageUri", alias = "image_uri")]
    pub image_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Environment {
    #[serde(alias = "variables")]
    pub variables: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct EnvironmentResponse {
    #[serde(alias = "variables")]
    pub variables: Option<HashMap<String, String>>,
    #[serde(alias = "error")]
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct EphemeralStorage {
    #[serde(alias = "size")]
    pub size: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct TracingConfig {
    #[serde(alias = "mode")]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateFunctionRequest {
    #[serde(alias = "functionName", alias = "function_name")]
    pub function_name: String,
    #[serde(default, alias = "runtime")]
    pub runtime: Option<String>,
    #[serde(alias = "role")]
    pub role: String,
    #[serde(default, alias = "handler")]
    pub handler: Option<String>,
    #[serde(default, alias = "code")]
    pub code: Option<FunctionCode>,
    #[serde(default, alias = "description")]
    pub description: Option<String>,
    #[serde(default, alias = "timeout")]
    pub timeout: Option<i32>,
    #[serde(default, alias = "memorySize", alias = "memory_size")]
    pub memory_size: Option<i32>,
    #[serde(default, alias = "publish")]
    pub publish: Option<bool>,
    #[serde(default, alias = "environment")]
    pub environment: Option<Environment>,
    #[serde(default, alias = "tags")]
    pub tags: Option<HashMap<String, String>>,
    #[serde(default, alias = "packageType", alias = "package_type")]
    pub package_type: Option<String>,
    #[serde(default, alias = "architectures")]
    pub architectures: Option<Vec<String>>,
    #[serde(default, alias = "ephemeralStorage", alias = "ephemeral_storage")]
    pub ephemeral_storage: Option<EphemeralStorage>,
    #[serde(default, alias = "tracingConfig", alias = "tracing_config")]
    pub tracing_config: Option<TracingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FunctionConfiguration {
    #[serde(alias = "functionName", alias = "function_name")]
    pub function_name: String,
    #[serde(alias = "functionArn", alias = "function_arn")]
    pub function_arn: String,
    #[serde(skip_serializing_if = "Option::is_none", alias = "runtime")]
    pub runtime: Option<String>,
    #[serde(alias = "role")]
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none", alias = "handler")]
    pub handler: Option<String>,
    #[serde(alias = "codeSize", alias = "code_size")]
    pub code_size: i64,
    #[serde(skip_serializing_if = "Option::is_none", alias = "description")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "timeout")]
    pub timeout: Option<i32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "memorySize",
        alias = "memory_size"
    )]
    pub memory_size: Option<i32>,
    #[serde(alias = "lastModified", alias = "last_modified")]
    pub last_modified: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "codeSha256",
        alias = "code_sha256"
    )]
    pub code_sha256: Option<String>,
    #[serde(alias = "version")]
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none", alias = "environment")]
    pub environment: Option<EnvironmentResponse>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "packageType",
        alias = "package_type"
    )]
    pub package_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "architectures")]
    pub architectures: Option<Vec<String>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "revisionId",
        alias = "revision_id"
    )]
    pub revision_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "state")]
    pub state: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "stateReason",
        alias = "state_reason"
    )]
    pub state_reason: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "stateReasonCode",
        alias = "state_reason_code"
    )]
    pub state_reason_code: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "ephemeralStorage",
        alias = "ephemeral_storage"
    )]
    pub ephemeral_storage: Option<EphemeralStorage>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "tracingConfig",
        alias = "tracing_config"
    )]
    pub tracing_config: Option<TracingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct FunctionCodeLocation {
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "repositoryType",
        alias = "repository_type"
    )]
    pub repository_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "location")]
    pub location: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "imageUri",
        alias = "image_uri"
    )]
    pub image_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetFunctionResponse {
    #[serde(skip_serializing_if = "Option::is_none", alias = "configuration")]
    pub configuration: Option<FunctionConfiguration>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "code")]
    pub code: Option<FunctionCodeLocation>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "tags")]
    pub tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListFunctionsResponse {
    #[serde(alias = "functions")]
    pub functions: Vec<FunctionConfiguration>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "nextMarker",
        alias = "next_marker"
    )]
    pub next_marker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteFunctionRequest {
    #[serde(alias = "functionName", alias = "function_name")]
    pub function_name: String,
    #[serde(default, alias = "qualifier")]
    pub qualifier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateEventSourceMappingRequest {
    #[serde(alias = "eventSourceArn", alias = "event_source_arn")]
    pub event_source_arn: String,
    #[serde(alias = "functionName", alias = "function_name")]
    pub function_name: String,
    #[serde(default, alias = "enabled")]
    pub enabled: Option<bool>,
    #[serde(default, alias = "batchSize", alias = "batch_size")]
    pub batch_size: Option<i32>,
    #[serde(default, alias = "startingPosition", alias = "starting_position")]
    pub starting_position: Option<String>,
    #[serde(
        default,
        alias = "startingPositionTimestamp",
        alias = "starting_position_timestamp"
    )]
    pub starting_position_timestamp: Option<i64>,
    #[serde(
        default,
        alias = "maximumBatchingWindowInSeconds",
        alias = "maximum_batching_window_in_seconds"
    )]
    pub maximum_batching_window_in_seconds: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EventSourceMappingConfiguration {
    #[serde(rename = "UUID", alias = "uuid")]
    pub uuid: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "batchSize",
        alias = "batch_size"
    )]
    pub batch_size: Option<i32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "eventSourceArn",
        alias = "event_source_arn"
    )]
    pub event_source_arn: Option<String>,
    #[serde(alias = "functionArn", alias = "function_arn")]
    pub function_arn: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "lastModified",
        alias = "last_modified"
    )]
    pub last_modified: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "state")]
    pub state: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "stateTransitionReason",
        alias = "state_transition_reason"
    )]
    pub state_transition_reason: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "startingPosition",
        alias = "starting_position"
    )]
    pub starting_position: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "maximumBatchingWindowInSeconds",
        alias = "maximum_batching_window_in_seconds"
    )]
    pub maximum_batching_window_in_seconds: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListEventSourceMappingsResponse {
    #[serde(alias = "eventSourceMappings", alias = "event_source_mappings")]
    pub event_source_mappings: Vec<EventSourceMappingConfiguration>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "nextMarker",
        alias = "next_marker"
    )]
    pub next_marker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFunction {
    pub configuration: FunctionConfiguration,
    pub code_location: Option<FunctionCodeLocation>,
    pub tags: HashMap<String, String>,
    pub raw_code: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LambdaStateSnapshot {
    pub functions: Vec<StoredFunction>,
    pub event_source_mappings: Vec<EventSourceMappingConfiguration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationResult {
    pub status_code: u16,
    pub payload: Vec<u8>,
    pub function_error: Option<String>,
    pub log_result: Option<String>,
    pub executed_version: String,
}
