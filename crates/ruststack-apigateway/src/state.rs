use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use ruststack_lambda::LambdaState;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ApiGatewayError {
    #[error("NotFoundException: {0}")]
    NotFound(String),
    #[error("BadRequestException: {0}")]
    BadRequest(String),
    #[error("ConflictException: {0}")]
    Conflict(String),
}

#[derive(Clone)]
pub struct ApiGatewayState {
    pub account_id: String,
    pub region: String,
    rest_apis: Arc<DashMap<String, Arc<RwLock<RestApi>>>>,
    lambda_state: Arc<RwLock<Option<Arc<LambdaState>>>>,
}

impl ApiGatewayState {
    pub fn new(account_id: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
            region: region.into(),
            rest_apis: Arc::new(DashMap::new()),
            lambda_state: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_lambda_state(&self, lambda: Arc<LambdaState>) {
        *self.lambda_state.write() = Some(lambda);
    }

    pub fn create_rest_api(&self, req: CreateRestApiRequest) -> Result<RestApi, ApiGatewayError> {
        let api_id = format!("{:x}", Uuid::new_v4().as_u128())[..10].to_string();
        let root_id = format!("{:x}", Uuid::new_v4().as_u128())[..10].to_string();
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;

        let mut resources = HashMap::new();
        let root_resource = Resource {
            id: root_id.clone(),
            parent_id: None,
            path_part: None,
            path: "/".to_string(),
            resource_methods: HashMap::new(),
        };
        resources.insert(root_id.clone(), root_resource);

        let api = RestApi {
            id: api_id.clone(),
            name: req.name,
            description: req.description,
            created_date: now,
            root_resource_id: root_id,
            resources,
            deployments: HashMap::new(),
            stages: HashMap::new(),
        };

        self.rest_apis
            .insert(api_id.clone(), Arc::new(RwLock::new(api.clone())));

        Ok(api)
    }

    pub fn get_rest_api(&self, api_id: &str) -> Result<RestApi, ApiGatewayError> {
        let api_entry = self
            .rest_apis
            .get(api_id)
            .ok_or_else(|| ApiGatewayError::NotFound(format!("RestApi {} not found", api_id)))?;
        let api = api_entry.read().clone();
        Ok(api)
    }

    pub fn get_rest_apis(&self) -> Result<GetRestApisResponse, ApiGatewayError> {
        let mut items = Vec::new();
        for item in self.rest_apis.iter() {
            let api = item.value().read();
            items.push(RestApiSummary {
                id: api.id.clone(),
                name: api.name.clone(),
                description: api.description.clone(),
                created_date: api.created_date,
            });
        }
        items.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(GetRestApisResponse { items })
    }

    pub fn delete_rest_api(&self, api_id: &str) -> Result<(), ApiGatewayError> {
        self.rest_apis
            .remove(api_id)
            .ok_or_else(|| ApiGatewayError::NotFound(format!("RestApi {} not found", api_id)))?;
        Ok(())
    }

    pub fn create_resource(
        &self,
        api_id: &str,
        parent_id: &str,
        path_part: &str,
    ) -> Result<Resource, ApiGatewayError> {
        let api_entry = self
            .rest_apis
            .get(api_id)
            .ok_or_else(|| ApiGatewayError::NotFound(format!("RestApi {} not found", api_id)))?;

        let mut api = api_entry.write();
        let parent_path = {
            let parent = api.resources.get(parent_id).ok_or_else(|| {
                ApiGatewayError::NotFound(format!("Parent resource {} not found", parent_id))
            })?;
            parent.path.clone()
        };

        let new_path = if parent_path == "/" {
            format!("/{}", path_part)
        } else {
            format!("{}/{}", parent_path, path_part)
        };

        let resource_id = format!("{:x}", Uuid::new_v4().as_u128())[..10].to_string();
        let resource = Resource {
            id: resource_id.clone(),
            parent_id: Some(parent_id.to_string()),
            path_part: Some(path_part.to_string()),
            path: new_path,
            resource_methods: HashMap::new(),
        };

        api.resources.insert(resource_id, resource.clone());
        Ok(resource)
    }

    pub fn get_resources(&self, api_id: &str) -> Result<GetResourcesResponse, ApiGatewayError> {
        let api_entry = self
            .rest_apis
            .get(api_id)
            .ok_or_else(|| ApiGatewayError::NotFound(format!("RestApi {} not found", api_id)))?;

        let api = api_entry.read();
        let mut items: Vec<Resource> = api.resources.values().cloned().collect();
        items.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(GetResourcesResponse { items })
    }

    pub fn get_resource(&self, api_id: &str, resource_id: &str) -> Result<Resource, ApiGatewayError> {
        let api_entry = self
            .rest_apis
            .get(api_id)
            .ok_or_else(|| ApiGatewayError::NotFound(format!("RestApi {} not found", api_id)))?;

        let api = api_entry.read();
        let res = api.resources.get(resource_id).ok_or_else(|| {
            ApiGatewayError::NotFound(format!("Resource {} not found", resource_id))
        })?;
        Ok(res.clone())
    }

    pub fn delete_resource(&self, api_id: &str, resource_id: &str) -> Result<(), ApiGatewayError> {
        let api_entry = self
            .rest_apis
            .get(api_id)
            .ok_or_else(|| ApiGatewayError::NotFound(format!("RestApi {} not found", api_id)))?;

        let mut api = api_entry.write();
        api.resources.remove(resource_id);
        Ok(())
    }

    pub fn put_method(
        &self,
        api_id: &str,
        resource_id: &str,
        http_method: &str,
        req: PutMethodRequest,
    ) -> Result<Method, ApiGatewayError> {
        let api_entry = self
            .rest_apis
            .get(api_id)
            .ok_or_else(|| ApiGatewayError::NotFound(format!("RestApi {} not found", api_id)))?;

        let mut api = api_entry.write();
        let res = api.resources.get_mut(resource_id).ok_or_else(|| {
            ApiGatewayError::NotFound(format!("Resource {} not found", resource_id))
        })?;

        let method = Method {
            http_method: http_method.to_uppercase(),
            authorization_type: req.authorization_type.or(Some("NONE".to_string())),
            method_integration: None,
            request_parameters: req.request_parameters,
        };

        res.resource_methods
            .insert(http_method.to_uppercase(), method.clone());
        Ok(method)
    }

    pub fn put_integration(
        &self,
        api_id: &str,
        resource_id: &str,
        http_method: &str,
        req: PutIntegrationRequest,
    ) -> Result<Integration, ApiGatewayError> {
        let api_entry = self
            .rest_apis
            .get(api_id)
            .ok_or_else(|| ApiGatewayError::NotFound(format!("RestApi {} not found", api_id)))?;

        let mut api = api_entry.write();
        let res = api.resources.get_mut(resource_id).ok_or_else(|| {
            ApiGatewayError::NotFound(format!("Resource {} not found", resource_id))
        })?;

        let method = res
            .resource_methods
            .get_mut(&http_method.to_uppercase())
            .ok_or_else(|| {
                ApiGatewayError::NotFound(format!("Method {} not found", http_method))
            })?;

        let integration = Integration {
            integration_type: req.integration_type,
            http_method: req.http_method,
            uri: req.uri,
            request_templates: req.request_templates,
            passthrough_behavior: Some("WHEN_NO_MATCH".to_string()),
            timeout_in_millis: Some(29000),
        };

        method.method_integration = Some(integration.clone());
        Ok(integration)
    }

    pub fn create_deployment(
        &self,
        api_id: &str,
        req: CreateDeploymentRequest,
    ) -> Result<Deployment, ApiGatewayError> {
        let api_entry = self
            .rest_apis
            .get(api_id)
            .ok_or_else(|| ApiGatewayError::NotFound(format!("RestApi {} not found", api_id)))?;

        let mut api = api_entry.write();
        let dep_id = format!("{:x}", Uuid::new_v4().as_u128())[..8].to_string();
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;

        let dep = Deployment {
            id: dep_id.clone(),
            description: req.description.clone(),
            created_date: now,
        };

        api.deployments.insert(dep_id.clone(), dep.clone());

        if let Some(stage_name) = req.stage_name {
            let stage = Stage {
                stage_name: stage_name.clone(),
                deployment_id: dep_id,
                description: req.stage_description,
                created_date: now,
                last_updated_date: now,
                variables: req.variables,
            };
            api.stages.insert(stage_name, stage);
        }

        Ok(dep)
    }

    pub fn create_stage(
        &self,
        api_id: &str,
        req: CreateStageRequest,
    ) -> Result<Stage, ApiGatewayError> {
        let api_entry = self
            .rest_apis
            .get(api_id)
            .ok_or_else(|| ApiGatewayError::NotFound(format!("RestApi {} not found", api_id)))?;

        let mut api = api_entry.write();
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        let stage = Stage {
            stage_name: req.stage_name.clone(),
            deployment_id: req.deployment_id,
            description: req.description,
            created_date: now,
            last_updated_date: now,
            variables: req.variables,
        };

        api.stages.insert(req.stage_name, stage.clone());
        Ok(stage)
    }

    pub fn get_stages(&self, api_id: &str) -> Result<GetStagesResponse, ApiGatewayError> {
        let api_entry = self
            .rest_apis
            .get(api_id)
            .ok_or_else(|| ApiGatewayError::NotFound(format!("RestApi {} not found", api_id)))?;

        let api = api_entry.read();
        let mut items: Vec<Stage> = api.stages.values().cloned().collect();
        items.sort_by(|a, b| a.stage_name.cmp(&b.stage_name));
        Ok(GetStagesResponse { item: items })
    }

    pub fn invoke_api(
        &self,
        api_id: &str,
        _stage: &str,
        http_method: &str,
        request_path: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<(u16, HashMap<String, String>, Vec<u8>), ApiGatewayError> {
        let api_entry = self
            .rest_apis
            .get(api_id)
            .ok_or_else(|| ApiGatewayError::NotFound(format!("RestApi {} not found", api_id)))?;

        let api = api_entry.read();

        // 1. Find matching resource path
        let norm_path = if request_path.is_empty() || request_path == "/" {
            "/".to_string()
        } else {
            format!("/{}", request_path.trim_matches('/'))
        };

        let target_resource = api.resources.values().find(|r| {
            if r.path == norm_path {
                return true;
            }
            // Simple parameterized route matching e.g. /users/{id}
            let r_segs: Vec<&str> = r.path.trim_matches('/').split('/').collect();
            let req_segs: Vec<&str> = norm_path.trim_matches('/').split('/').collect();
            if r_segs.len() != req_segs.len() {
                return false;
            }
            for (rs, qs) in r_segs.iter().zip(req_segs.iter()) {
                if rs.starts_with('{') && rs.ends_with('}') {
                    continue;
                }
                if rs != qs {
                    return false;
                }
            }
            true
        });

        let resource = match target_resource {
            Some(r) => r,
            None => {
                let mut res_headers = HashMap::new();
                res_headers.insert("content-type".to_string(), "application/json".to_string());
                let err_body = serde_json::json!({
                    "message": format!("Missing Authentication Token for path {}", norm_path)
                })
                .to_string();
                return Ok((403, res_headers, err_body.into_bytes()));
            }
        };

        let method = match resource.resource_methods.get(&http_method.to_uppercase()) {
            Some(m) => m,
            None => {
                let mut res_headers = HashMap::new();
                res_headers.insert("content-type".to_string(), "application/json".to_string());
                let err_body = serde_json::json!({
                    "message": format!("Method {} not allowed on {}", http_method, norm_path)
                })
                .to_string();
                return Ok((405, res_headers, err_body.into_bytes()));
            }
        };

        let integration = match &method.method_integration {
            Some(i) => i,
            None => {
                let mut res_headers = HashMap::new();
                res_headers.insert("content-type".to_string(), "application/json".to_string());
                return Ok((
                    200,
                    res_headers,
                    b"{\"message\":\"Mock integration default response\"}".to_vec(),
                ));
            }
        };

        match integration.integration_type {
            IntegrationType::Mock => {
                let mut res_headers = HashMap::new();
                res_headers.insert("content-type".to_string(), "application/json".to_string());
                let resp_body = if let Some(ref tpls) = integration.request_templates {
                    tpls.get("application/json")
                        .cloned()
                        .unwrap_or_else(|| "{\"statusCode\": 200}".to_string())
                } else {
                    "{\"statusCode\": 200}".to_string()
                };
                Ok((200, res_headers, resp_body.into_bytes()))
            }
            IntegrationType::AwsProxy | IntegrationType::Aws => {
                let uri_str = integration.uri.as_deref().unwrap_or("");
                let lambda_arn = if let Some(idx) = uri_str.find("/functions/") {
                    let rem = &uri_str[idx + 11..];
                    if let Some(end_idx) = rem.find("/invocations") {
                        &rem[..end_idx]
                    } else {
                        rem
                    }
                } else {
                    uri_str
                };

                let lambda_opt = self.lambda_state.read().clone();
                if let Some(lambda) = lambda_opt {
                    let lambda_event = serde_json::json!({
                        "path": norm_path,
                        "httpMethod": http_method.to_uppercase(),
                        "headers": headers,
                        "body": String::from_utf8_lossy(body).to_string(),
                        "isBase64Encoded": false,
                        "requestContext": {
                            "apiId": api_id,
                            "httpMethod": http_method.to_uppercase(),
                            "path": norm_path,
                            "stage": _stage,
                            "accountId": self.account_id
                        }
                    });

                    let payload = serde_json::to_vec(&lambda_event).unwrap_or_default();
                    match lambda.invoke_function(
                        lambda_arn,
                        Some(payload),
                        Some(ruststack_lambda::types::InvocationType::RequestResponse),
                    ) {
                        Ok(inv) => {
                            if let Ok(proxy_res) = serde_json::from_slice::<serde_json::Value>(&inv.payload) {
                                let status = proxy_res
                                    .get("statusCode")
                                    .and_then(|s| s.as_u64())
                                    .unwrap_or(200) as u16;

                                let mut out_headers = HashMap::new();
                                if let Some(h) = proxy_res.get("headers").and_then(|v| v.as_object()) {
                                    for (k, v) in h {
                                        if let Some(s) = v.as_str() {
                                            out_headers.insert(k.clone(), s.to_string());
                                        }
                                    }
                                }

                                let out_body = if let Some(b) = proxy_res.get("body").and_then(|v| v.as_str()) {
                                    b.as_bytes().to_vec()
                                } else {
                                    inv.payload
                                };

                                Ok((status, out_headers, out_body))
                            } else {
                                let mut out_headers = HashMap::new();
                                out_headers.insert("content-type".to_string(), "application/json".to_string());
                                Ok((200, out_headers, inv.payload))
                            }
                        }
                        Err(e) => {
                            let mut out_headers = HashMap::new();
                            out_headers.insert("content-type".to_string(), "application/json".to_string());
                            let err_str = format!("{{\"message\": \"Lambda invocation error: {}\"}}", e);
                            Ok((502, out_headers, err_str.into_bytes()))
                        }
                    }
                } else {
                    let mut out_headers = HashMap::new();
                    out_headers.insert("content-type".to_string(), "application/json".to_string());
                    Ok((
                        200,
                        out_headers,
                        b"{\"message\": \"Lambda service not wired\"}".to_vec(),
                    ))
                }
            }
            _ => {
                let mut res_headers = HashMap::new();
                res_headers.insert("content-type".to_string(), "application/json".to_string());
                Ok((200, res_headers, b"{\"status\":\"OK\"}".to_vec()))
            }
        }
    }

    pub fn export_snapshot(&self) -> ApiGatewayStateSnapshot {
        let mut map = HashMap::new();
        for item in self.rest_apis.iter() {
            let api = item.value().read().clone();
            map.insert(item.key().clone(), api);
        }
        ApiGatewayStateSnapshot { rest_apis: map }
    }

    pub fn import_snapshot(&self, snapshot: ApiGatewayStateSnapshot) {
        self.rest_apis.clear();
        for (k, v) in snapshot.rest_apis {
            self.rest_apis.insert(k, Arc::new(RwLock::new(v)));
        }
    }

    pub fn reset(&self) {
        self.rest_apis.clear();
    }
}
