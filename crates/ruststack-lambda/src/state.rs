use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum LambdaError {
    #[error("ResourceNotFoundException: {0}")]
    NotFound(String),
    #[error("ResourceConflictException: {0}")]
    Conflict(String),
    #[error("InvalidParameterValueException: {0}")]
    InvalidParameter(String),
    #[error("InvalidRequestContentException: {0}")]
    InvalidRequestContent(String),
    #[error("ServiceException: {0}")]
    Service(String),
}

#[derive(Clone)]
pub struct LambdaState {
    account_id: String,
    region: String,
    functions: Arc<DashMap<String, StoredFunction>>,
    event_source_mappings: Arc<DashMap<String, EventSourceMappingConfiguration>>,
    sqs_engine: Arc<parking_lot::RwLock<Option<Arc<ruststack_sqs::SqsEngine>>>>,
}

impl LambdaState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            functions: Arc::new(DashMap::new()),
            event_source_mappings: Arc::new(DashMap::new()),
            sqs_engine: Arc::new(parking_lot::RwLock::new(None)),
        }
    }

    pub fn set_sqs_engine(&self, engine: Arc<ruststack_sqs::SqsEngine>) {
        *self.sqs_engine.write() = Some(engine);
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn function_arn(&self, name: &str) -> String {
        if name.starts_with("arn:aws:lambda:") {
            name.to_string()
        } else {
            format!(
                "arn:aws:lambda:{}:{}:function:{}",
                self.region, self.account_id, name
            )
        }
    }

    pub fn normalize_function_name<'a>(&self, name_or_arn: &'a str) -> &'a str {
        let after_arn = if let Some(idx) = name_or_arn.find(":function:") {
            &name_or_arn[idx + 10..]
        } else {
            name_or_arn
        };

        if let Some(idx) = after_arn.find(':') {
            &after_arn[..idx]
        } else {
            after_arn
        }
    }

    pub fn create_function(
        &self,
        req: CreateFunctionRequest,
    ) -> Result<FunctionConfiguration, LambdaError> {
        let name = req.function_name.trim();
        if name.is_empty() {
            return Err(LambdaError::InvalidParameter(
                "FunctionName cannot be empty".to_string(),
            ));
        }

        if self.functions.contains_key(name) {
            return Err(LambdaError::Conflict(format!(
                "Function already exist: {}",
                name
            )));
        }

        let arn = self.function_arn(name);
        let now = Utc::now().to_rfc3339();

        let (code_size, raw_code) = match &req.code {
            Some(code) => {
                if let Some(ref zip) = code.zip_file {
                    match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, zip) {
                        Ok(bytes) => (bytes.len() as i64, Some(bytes)),
                        Err(_) => (zip.len() as i64, Some(zip.as_bytes().to_vec())),
                    }
                } else {
                    (0, None)
                }
            }
            None => (0, None),
        };

        let env_resp = req.environment.map(|e| EnvironmentResponse {
            variables: e.variables,
            error: None,
        });

        let config = FunctionConfiguration {
            function_name: name.to_string(),
            function_arn: arn.clone(),
            runtime: req.runtime.or_else(|| Some("provided.al2023".to_string())),
            role: req.role,
            handler: req.handler.or_else(|| Some("bootstrap".to_string())),
            code_size,
            description: req.description,
            timeout: req.timeout.or(Some(3)),
            memory_size: req.memory_size.or(Some(128)),
            last_modified: now,
            code_sha256: Some(Uuid::new_v4().simple().to_string()),
            version: "$LATEST".to_string(),
            environment: env_resp,
            package_type: req.package_type.or_else(|| Some("Zip".to_string())),
            architectures: req
                .architectures
                .or_else(|| Some(vec!["x86_64".to_string()])),
            revision_id: Some(Uuid::new_v4().to_string()),
            state: Some("Active".to_string()),
            state_reason: None,
            state_reason_code: None,
            ephemeral_storage: req.ephemeral_storage,
            tracing_config: req.tracing_config,
        };

        let code_location = req.code.map(|c| FunctionCodeLocation {
            repository_type: Some("S3".to_string()),
            location: Some(format!(
                "https://prod-04-2014-tasks.s3.amazonaws.com/snapshots/{}/{}",
                self.account_id, name
            )),
            image_uri: c.image_uri,
        });

        let stored = StoredFunction {
            configuration: config.clone(),
            code_location,
            tags: req.tags.unwrap_or_default(),
            raw_code,
        };

        self.functions.insert(name.to_string(), stored);
        Ok(config)
    }

    pub fn get_function(&self, name_or_arn: &str) -> Result<GetFunctionResponse, LambdaError> {
        let name = self.normalize_function_name(name_or_arn);
        let stored = self
            .functions
            .get(name)
            .ok_or_else(|| LambdaError::NotFound(format!("Function not found: {}", name_or_arn)))?;

        Ok(GetFunctionResponse {
            configuration: Some(stored.configuration.clone()),
            code: stored.code_location.clone(),
            tags: Some(stored.tags.clone()),
        })
    }

    pub fn get_function_configuration(
        &self,
        name_or_arn: &str,
    ) -> Result<FunctionConfiguration, LambdaError> {
        let name = self.normalize_function_name(name_or_arn);
        let stored = self
            .functions
            .get(name)
            .ok_or_else(|| LambdaError::NotFound(format!("Function not found: {}", name_or_arn)))?;

        Ok(stored.configuration.clone())
    }

    pub fn delete_function(&self, req: DeleteFunctionRequest) -> Result<(), LambdaError> {
        let name = self.normalize_function_name(&req.function_name);
        if self.functions.remove(name).is_some() {
            Ok(())
        } else {
            Err(LambdaError::NotFound(format!(
                "Function not found: {}",
                req.function_name
            )))
        }
    }

    pub fn list_functions(
        &self,
        _marker: Option<String>,
        max_items: Option<usize>,
    ) -> Result<ListFunctionsResponse, LambdaError> {
        let mut list: Vec<FunctionConfiguration> = self
            .functions
            .iter()
            .map(|item| item.configuration.clone())
            .collect();

        list.sort_by(|a, b| a.function_name.cmp(&b.function_name));

        let limit = max_items.unwrap_or(50);
        if list.len() > limit {
            list.truncate(limit);
        }

        Ok(ListFunctionsResponse {
            functions: list,
            next_marker: None,
        })
    }

    pub fn invoke_function(
        &self,
        name_or_arn: &str,
        payload: Option<Vec<u8>>,
        invocation_type: Option<InvocationType>,
    ) -> Result<InvocationResult, LambdaError> {
        let name = self.normalize_function_name(name_or_arn);
        if !self.functions.contains_key(name) {
            return Err(LambdaError::NotFound(format!(
                "Function not found: {}",
                name_or_arn
            )));
        }

        let inv_type = invocation_type.unwrap_or(InvocationType::RequestResponse);

        match inv_type {
            InvocationType::DryRun => Ok(InvocationResult {
                status_code: 204,
                payload: Vec::new(),
                function_error: None,
                log_result: None,
                executed_version: "$LATEST".to_string(),
            }),
            InvocationType::Event => Ok(InvocationResult {
                status_code: 202,
                payload: Vec::new(),
                function_error: None,
                log_result: None,
                executed_version: "$LATEST".to_string(),
            }),
            InvocationType::RequestResponse => {
                let payload_bytes = payload.unwrap_or_default();

                if payload_bytes.is_empty() {
                    let default_body = serde_json::json!({
                        "statusCode": 200,
                        "body": "Hello from Lambda!"
                    });
                    return Ok(InvocationResult {
                        status_code: 200,
                        payload: serde_json::to_vec(&default_body).unwrap_or_default(),
                        function_error: None,
                        log_result: None,
                        executed_version: "$LATEST".to_string(),
                    });
                }

                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&payload_bytes) {
                    if let Some(err_obj) = val.get("__error").or_else(|| val.get("error")) {
                        let err_payload = if err_obj.is_object() {
                            serde_json::to_vec(err_obj).unwrap_or_default()
                        } else {
                            serde_json::to_vec(&serde_json::json!({
                                "errorType": "Error",
                                "errorMessage": err_obj.to_string()
                            }))
                            .unwrap_or_default()
                        };
                        return Ok(InvocationResult {
                            status_code: 200,
                            payload: err_payload,
                            function_error: Some("Unhandled".to_string()),
                            log_result: None,
                            executed_version: "$LATEST".to_string(),
                        });
                    }

                    if let Some(mock_resp) = val
                        .get("mock_response")
                        .or_else(|| val.get("mockResponse"))
                        .or_else(|| val.get("response"))
                    {
                        let resp_bytes = match mock_resp {
                            serde_json::Value::String(s) => s.as_bytes().to_vec(),
                            other => serde_json::to_vec(other).unwrap_or_default(),
                        };
                        return Ok(InvocationResult {
                            status_code: 200,
                            payload: resp_bytes,
                            function_error: None,
                            log_result: None,
                            executed_version: "$LATEST".to_string(),
                        });
                    }
                }

                // Default mock execution: echo payload back
                Ok(InvocationResult {
                    status_code: 200,
                    payload: payload_bytes,
                    function_error: None,
                    log_result: None,
                    executed_version: "$LATEST".to_string(),
                })
            }
        }
    }

    pub fn create_event_source_mapping(
        &self,
        req: CreateEventSourceMappingRequest,
    ) -> Result<EventSourceMappingConfiguration, LambdaError> {
        let fn_name = self.normalize_function_name(&req.function_name);
        let stored = self.functions.get(fn_name).ok_or_else(|| {
            LambdaError::NotFound(format!("Function not found: {}", req.function_name))
        })?;

        let uuid = Uuid::new_v4().to_string();
        let now_sec = Utc::now().timestamp_millis() as f64 / 1000.0;

        let mapping = EventSourceMappingConfiguration {
            uuid: uuid.clone(),
            batch_size: req.batch_size.or(Some(10)),
            event_source_arn: Some(req.event_source_arn),
            function_arn: stored.configuration.function_arn.clone(),
            last_modified: Some(now_sec),
            state: Some(
                if req.enabled.unwrap_or(true) {
                    "Enabled"
                } else {
                    "Disabled"
                }
                .to_string(),
            ),
            state_transition_reason: Some("User action".to_string()),
            starting_position: req.starting_position.or_else(|| Some("LATEST".to_string())),
            maximum_batching_window_in_seconds: req.maximum_batching_window_in_seconds,
        };

        self.event_source_mappings.insert(uuid, mapping.clone());
        Ok(mapping)
    }

    pub fn get_event_source_mapping(
        &self,
        uuid: &str,
    ) -> Result<EventSourceMappingConfiguration, LambdaError> {
        self.event_source_mappings
            .get(uuid)
            .map(|m| m.clone())
            .ok_or_else(|| LambdaError::NotFound(format!("EventSourceMapping not found: {}", uuid)))
    }

    pub fn delete_event_source_mapping(
        &self,
        uuid: &str,
    ) -> Result<EventSourceMappingConfiguration, LambdaError> {
        self.event_source_mappings
            .remove(uuid)
            .map(|(_, m)| m)
            .ok_or_else(|| LambdaError::NotFound(format!("EventSourceMapping not found: {}", uuid)))
    }

    pub fn list_event_source_mappings(
        &self,
        event_source_arn: Option<&str>,
        function_name: Option<&str>,
    ) -> Result<ListEventSourceMappingsResponse, LambdaError> {
        let target_fn_name = function_name.map(|f| self.normalize_function_name(f));

        let mut list = Vec::new();
        for item in self.event_source_mappings.iter() {
            let mapping = item.value();

            if let Some(src) = event_source_arn {
                if mapping.event_source_arn.as_deref() != Some(src) {
                    continue;
                }
            }

            if let Some(target) = target_fn_name {
                let fn_in_mapping = self.normalize_function_name(&mapping.function_arn);
                if fn_in_mapping != target {
                    continue;
                }
            }

            list.push(mapping.clone());
        }

        list.sort_by(|a, b| a.uuid.cmp(&b.uuid));

        Ok(ListEventSourceMappingsResponse {
            event_source_mappings: list,
            next_marker: None,
        })
    }

    pub fn export_snapshot(&self) -> LambdaStateSnapshot {
        LambdaStateSnapshot {
            functions: self
                .functions
                .iter()
                .map(|item| item.value().clone())
                .collect(),
            event_source_mappings: self
                .event_source_mappings
                .iter()
                .map(|item| item.value().clone())
                .collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: LambdaStateSnapshot) {
        self.functions.clear();
        self.event_source_mappings.clear();

        for f in snapshot.functions {
            self.functions
                .insert(f.configuration.function_name.clone(), f);
        }
        for m in snapshot.event_source_mappings {
            self.event_source_mappings.insert(m.uuid.clone(), m);
        }
    }

    pub async fn poll_event_sources_once(&self) -> usize {
        let mut processed = 0;
        let mappings: Vec<EventSourceMappingConfiguration> = self
            .event_source_mappings
            .iter()
            .map(|item| item.value().clone())
            .collect();

        let sqs_opt = self.sqs_engine.read().clone();

        for mapping in mappings {
            if mapping.state.as_deref() != Some("Enabled") {
                continue;
            }

            if let Some(ref src_arn) = mapping.event_source_arn {
                if src_arn.starts_with("arn:aws:sqs:") || src_arn.contains(":sqs:") {
                    if let Some(ref sqs) = sqs_opt {
                        let batch_size = mapping.batch_size.unwrap_or(10) as u32;
                        if let Ok(msgs) = sqs
                            .receive_message(src_arn, batch_size, Some(30), Some(0))
                            .await
                        {
                            if !msgs.is_empty() {
                                let records: Vec<serde_json::Value> = msgs
                                    .iter()
                                    .map(|m| {
                                        serde_json::json!({
                                            "messageId": m.message_id,
                                            "receiptHandle": m.receipt_handle,
                                            "body": m.body,
                                            "attributes": m.attributes,
                                            "md5OfBody": m.md5_of_body,
                                            "eventSource": "aws:sqs",
                                            "eventSourceARN": src_arn,
                                            "awsRegion": self.region
                                        })
                                    })
                                    .collect();

                                let event_payload = serde_json::json!({ "Records": records });
                                let payload_bytes =
                                    serde_json::to_vec(&event_payload).unwrap_or_default();

                                if self
                                    .invoke_function(
                                        &mapping.function_arn,
                                        Some(payload_bytes),
                                        Some(InvocationType::Event),
                                    )
                                    .is_ok()
                                {
                                    for m in &msgs {
                                        let _ = sqs.delete_message(src_arn, &m.receipt_handle);
                                    }
                                    processed += msgs.len();
                                }
                            }
                        }
                    }
                }
            }
        }

        processed
    }

    pub fn start_poller(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
            loop {
                interval.tick().await;
                let _ = self.poll_event_sources_once().await;
            }
        })
    }

    pub fn reset(&self) {
        self.functions.clear();
        self.event_source_mappings.clear();
    }
}
