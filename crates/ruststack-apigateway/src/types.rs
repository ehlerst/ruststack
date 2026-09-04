use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IntegrationType {
    #[default]
    Mock,
    AwsProxy,
    Aws,
    Http,
    HttpProxy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Integration {
    #[serde(rename = "type")]
    pub integration_type: IntegrationType,
    pub http_method: Option<String>,
    pub uri: Option<String>, // e.g. arn:aws:apigateway:us-east-1:lambda:path/2015-03-31/functions/arn:aws:lambda:.../invocations
    pub request_templates: Option<HashMap<String, String>>,
    pub passthrough_behavior: Option<String>,
    pub timeout_in_millis: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Method {
    pub http_method: String,
    pub authorization_type: Option<String>,
    pub method_integration: Option<Integration>,
    pub request_parameters: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub id: String,
    pub parent_id: Option<String>,
    pub path_part: Option<String>,
    pub path: String,
    pub resource_methods: HashMap<String, Method>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RestApi {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_date: f64,
    pub root_resource_id: String,
    pub resources: HashMap<String, Resource>,
    pub deployments: HashMap<String, Deployment>,
    pub stages: HashMap<String, Stage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    pub id: String,
    pub description: Option<String>,
    pub created_date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    pub stage_name: String,
    pub deployment_id: String,
    pub description: Option<String>,
    pub created_date: f64,
    pub last_updated_date: f64,
    pub variables: Option<HashMap<String, String>>,
}

// ----------------------------------------------------------------------------
// Request / Response payloads
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRestApiRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetRestApisResponse {
    pub items: Vec<RestApiSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestApiSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResourceRequest {
    pub path_part: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetResourcesResponse {
    pub items: Vec<Resource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutMethodRequest {
    pub authorization_type: Option<String>,
    pub request_parameters: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutIntegrationRequest {
    #[serde(rename = "type")]
    pub integration_type: IntegrationType,
    pub http_method: Option<String>,
    pub uri: Option<String>,
    pub request_templates: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeploymentRequest {
    pub stage_name: Option<String>,
    pub description: Option<String>,
    pub stage_description: Option<String>,
    pub variables: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStageRequest {
    pub stage_name: String,
    pub deployment_id: String,
    pub description: Option<String>,
    pub variables: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetStagesResponse {
    pub item: Vec<Stage>,
}

// ----------------------------------------------------------------------------
// Snapshot
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiGatewayStateSnapshot {
    pub rest_apis: HashMap<String, RestApi>,
}
