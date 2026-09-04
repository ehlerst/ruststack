use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use ruststack_dynamodb::DynamoDbEngine;
use ruststack_iam::IamState;
use ruststack_s3::S3Storage;
use ruststack_sns::SnsEngine;
use ruststack_sqs::SqsEngine;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum CloudFormationError {
    #[error("AlreadyExistsException: Stack [{0}] already exists")]
    StackAlreadyExists(String),
    #[error("ValidationError: Stack with id {0} does not exist")]
    StackNotFound(String),
    #[error("ValidationError: Template format error: {0}")]
    TemplateFormatError(String),
}

#[derive(Clone)]
pub struct CloudFormationState {
    pub account_id: String,
    pub region: String,
    stacks: Arc<DashMap<String, Arc<RwLock<StoredStack>>>>,
    s3_storage: Arc<RwLock<Option<Arc<dyn S3Storage>>>>,
    sqs_engine: Arc<RwLock<Option<Arc<SqsEngine>>>>,
    sns_engine: Arc<RwLock<Option<Arc<SnsEngine>>>>,
    dynamodb_engine: Arc<RwLock<Option<Arc<DynamoDbEngine>>>>,
    iam_state: Arc<RwLock<Option<Arc<IamState>>>>,
}

impl CloudFormationState {
    pub fn new(account_id: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
            region: region.into(),
            stacks: Arc::new(DashMap::new()),
            s3_storage: Arc::new(RwLock::new(None)),
            sqs_engine: Arc::new(RwLock::new(None)),
            sns_engine: Arc::new(RwLock::new(None)),
            dynamodb_engine: Arc::new(RwLock::new(None)),
            iam_state: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_services(
        &self,
        s3: Option<Arc<dyn S3Storage>>,
        sqs: Option<Arc<SqsEngine>>,
        sns: Option<Arc<SnsEngine>>,
        dynamodb: Option<Arc<DynamoDbEngine>>,
        iam: Option<Arc<IamState>>,
    ) {
        *self.s3_storage.write() = s3;
        *self.sqs_engine.write() = sqs;
        *self.sns_engine.write() = sns;
        *self.dynamodb_engine.write() = dynamodb;
        *self.iam_state.write() = iam;
    }

    pub fn format_stack_id(&self, stack_name: &str) -> String {
        format!(
            "arn:aws:cloudformation:{}:{}:stack/{}/{}",
            self.region,
            self.account_id,
            stack_name,
            Uuid::new_v4()
        )
    }

    pub fn create_stack(
        &self,
        stack_name: String,
        template_body: String,
        parameters: Vec<Parameter>,
    ) -> Result<String, CloudFormationError> {
        if self.stacks.contains_key(&stack_name) {
            let existing = self.stacks.get(&stack_name).unwrap().read().clone();
            if existing.status != StackStatus::DeleteComplete {
                return Err(CloudFormationError::StackAlreadyExists(stack_name));
            }
        }

        let stack_id = self.format_stack_id(&stack_name);
        let now = Utc::now();

        // Normalize shorthand tags like !Ref, !Sub, !GetAtt for YAML parsing
        let normalized_template = template_body
            .lines()
            .map(|line| {
                if let Some(pos) = line.find("!Ref ") {
                    let mut l = line.to_string();
                    l.replace_range(pos..pos + 5, "{ Ref: ");
                    l.push_str(" }");
                    l
                } else if let Some(pos) = line.find("!Sub ") {
                    let mut l = line.to_string();
                    l.replace_range(pos..pos + 5, "{ \"Fn::Sub\": ");
                    l.push_str(" }");
                    l
                } else if let Some(pos) = line.find("!GetAtt ") {
                    let mut l = line.to_string();
                    l.replace_range(pos..pos + 8, "{ \"Fn::GetAtt\": ");
                    l.push_str(" }");
                    l
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("\n");

        // Parse template as JSON or YAML
        let template_val: serde_json::Value = if let Ok(json_v) = serde_json::from_str(&template_body) {
            json_v
        } else if let Ok(yaml_v) = serde_yaml::from_str::<serde_json::Value>(&normalized_template) {
            yaml_v
        } else if let Ok(yaml_raw) = serde_yaml::from_str::<serde_yaml::Value>(&normalized_template) {
            serde_json::to_value(yaml_raw).map_err(|e| {
                CloudFormationError::TemplateFormatError(format!("YAML conversion error: {}", e))
            })?
        } else {
            return Err(CloudFormationError::TemplateFormatError(
                "Failed to parse template as JSON or YAML".to_string(),
            ));
        };

        let mut param_map = HashMap::new();
        for p in &parameters {
            param_map.insert(p.parameter_key.clone(), p.parameter_value.clone());
        }

        let mut events = Vec::new();
        events.push(StackEvent {
            event_id: Uuid::new_v4().to_string(),
            stack_id: stack_id.clone(),
            stack_name: stack_name.clone(),
            logical_resource_id: stack_name.clone(),
            physical_resource_id: stack_id.clone(),
            resource_type: "AWS::CloudFormation::Stack".to_string(),
            timestamp: now,
            resource_status: "CREATE_IN_PROGRESS".to_string(),
            resource_status_reason: Some("User Initiated".to_string()),
        });

        // Provision resources
        let mut resources = Vec::new();
        let mut resource_map = HashMap::new();

        if let Some(res_obj) = template_val.get("Resources").and_then(|v| v.as_object()) {
            for (logical_id, def) in res_obj {
                let res_type = def.get("Type").and_then(|v| v.as_str()).unwrap_or("");
                let props = def.get("Properties").cloned().unwrap_or(serde_json::json!({}));

                let physical_id = match res_type {
                    "AWS::S3::Bucket" => {
                        let bucket_name = props
                            .get("BucketName")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| format!("{}-{}", stack_name.to_lowercase(), logical_id.to_lowercase()));
                        if let Some(s3) = self.s3_storage.read().clone() {
                            let _ = s3.create_bucket(&bucket_name, &self.region);
                        }
                        bucket_name
                    }
                    "AWS::SQS::Queue" => {
                        let q_name = props
                            .get("QueueName")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| format!("{}-{}", stack_name, logical_id));
                        if let Some(sqs) = self.sqs_engine.read().clone() {
                            let _ = sqs.create_queue(&q_name, Some(HashMap::new()));
                        }
                        format!("https://sqs.{}.amazonaws.com/{}/{}", self.region, self.account_id, q_name)
                    }
                    "AWS::SNS::Topic" => {
                        let topic_name = props
                            .get("TopicName")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| format!("{}-{}", stack_name, logical_id));
                        if let Some(sns) = self.sns_engine.read().clone() {
                            let _ = sns.create_topic(&topic_name, Some(HashMap::new()));
                        }
                        format!("arn:aws:sns:{}:{}:{}", self.region, self.account_id, topic_name)
                    }
                    "AWS::DynamoDB::Table" => {
                        let table_name = props
                            .get("TableName")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| format!("{}-{}", stack_name, logical_id));
                        table_name
                    }
                    _ => format!("{}-{}", stack_name, logical_id),
                };

                resource_map.insert(logical_id.clone(), physical_id.clone());
                resources.push(StackResource {
                    logical_resource_id: logical_id.clone(),
                    physical_resource_id: physical_id.clone(),
                    resource_type: res_type.to_string(),
                    resource_status: "CREATE_COMPLETE".to_string(),
                    timestamp: Utc::now(),
                });

                events.push(StackEvent {
                    event_id: Uuid::new_v4().to_string(),
                    stack_id: stack_id.clone(),
                    stack_name: stack_name.clone(),
                    logical_resource_id: logical_id.clone(),
                    physical_resource_id: physical_id,
                    resource_type: res_type.to_string(),
                    timestamp: Utc::now(),
                    resource_status: "CREATE_COMPLETE".to_string(),
                    resource_status_reason: None,
                });
            }
        }

        // Process Outputs
        let mut outputs = Vec::new();
        if let Some(out_obj) = template_val.get("Outputs").and_then(|v| v.as_object()) {
            for (out_key, out_def) in out_obj {
                let desc = out_def.get("Description").and_then(|v| v.as_str()).map(String::from);
                let exp = out_def
                    .get("Export")
                    .and_then(|v| v.get("Name"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let raw_val = out_def.get("Value");
                let resolved_val = if let Some(s) = raw_val.and_then(|v| v.as_str()) {
                    s.to_string()
                } else if let Some(ref_val) = raw_val.and_then(|v| v.get("Ref")).and_then(|v| v.as_str()) {
                    if let Some(phys) = resource_map.get(ref_val) {
                        phys.clone()
                    } else if let Some(param) = param_map.get(ref_val) {
                        param.clone()
                    } else {
                        ref_val.to_string()
                    }
                } else {
                    raw_val.map(|v| v.to_string()).unwrap_or_default()
                };

                outputs.push(Output {
                    output_key: out_key.clone(),
                    output_value: resolved_val,
                    description: desc,
                    export_name: exp,
                });
            }
        }

        events.push(StackEvent {
            event_id: Uuid::new_v4().to_string(),
            stack_id: stack_id.clone(),
            stack_name: stack_name.clone(),
            logical_resource_id: stack_name.clone(),
            physical_resource_id: stack_id.clone(),
            resource_type: "AWS::CloudFormation::Stack".to_string(),
            timestamp: Utc::now(),
            resource_status: "CREATE_COMPLETE".to_string(),
            resource_status_reason: None,
        });

        let stored = StoredStack {
            stack_id: stack_id.clone(),
            stack_name: stack_name.clone(),
            template_body,
            status: StackStatus::CreateComplete,
            status_reason: None,
            creation_time: now,
            last_updated_time: None,
            parameters,
            outputs,
            resources,
            events,
        };

        self.stacks.insert(stack_name.clone(), Arc::new(RwLock::new(stored.clone())));
        self.stacks.insert(stack_id.clone(), Arc::new(RwLock::new(stored)));

        Ok(stack_id)
    }

    pub fn describe_stacks(&self, stack_name_or_id: Option<&str>) -> Result<Vec<StoredStack>, CloudFormationError> {
        if let Some(name_or_id) = stack_name_or_id {
            if let Some(entry) = self.stacks.get(name_or_id) {
                return Ok(vec![entry.read().clone()]);
            } else {
                return Err(CloudFormationError::StackNotFound(name_or_id.to_string()));
            }
        }

        let mut seen = std::collections::HashSet::new();
        let mut list = Vec::new();
        for item in self.stacks.iter() {
            let stack = item.value().read().clone();
            if seen.insert(stack.stack_id.clone()) && stack.status != StackStatus::DeleteComplete {
                list.push(stack);
            }
        }
        list.sort_by(|a, b| b.creation_time.cmp(&a.creation_time));
        Ok(list)
    }

    pub fn describe_stack_resources(&self, stack_name: &str) -> Result<Vec<StackResource>, CloudFormationError> {
        let entry = self.stacks.get(stack_name).ok_or_else(|| {
            CloudFormationError::StackNotFound(stack_name.to_string())
        })?;
        let res = entry.read().resources.clone();
        Ok(res)
    }

    pub fn describe_stack_events(&self, stack_name: &str) -> Result<Vec<StackEvent>, CloudFormationError> {
        let entry = self.stacks.get(stack_name).ok_or_else(|| {
            CloudFormationError::StackNotFound(stack_name.to_string())
        })?;
        let mut events = entry.read().events.clone();
        events.reverse();
        Ok(events)
    }

    pub fn get_template(&self, stack_name: &str) -> Result<String, CloudFormationError> {
        let entry = self.stacks.get(stack_name).ok_or_else(|| {
            CloudFormationError::StackNotFound(stack_name.to_string())
        })?;
        let body = entry.read().template_body.clone();
        Ok(body)
    }

    pub fn delete_stack(&self, stack_name: &str) -> Result<(), CloudFormationError> {
        if let Some(entry) = self.stacks.get(stack_name) {
            let mut stack = entry.write();
            stack.status = StackStatus::DeleteComplete;
            let stack_id = stack.stack_id.clone();
            let name = stack.stack_name.clone();
            stack.events.push(StackEvent {
                event_id: Uuid::new_v4().to_string(),
                stack_id: stack_id.clone(),
                stack_name: name.clone(),
                logical_resource_id: name,
                physical_resource_id: stack_id,
                resource_type: "AWS::CloudFormation::Stack".to_string(),
                timestamp: Utc::now(),
                resource_status: "DELETE_COMPLETE".to_string(),
                resource_status_reason: None,
            });
        }
        Ok(())
    }

    pub fn export_snapshot(&self) -> CloudFormationStateSnapshot {
        let mut map = HashMap::new();
        let mut seen = std::collections::HashSet::new();
        for item in self.stacks.iter() {
            let stack = item.value().read().clone();
            if seen.insert(stack.stack_id.clone()) {
                map.insert(stack.stack_name.clone(), stack);
            }
        }
        CloudFormationStateSnapshot { stacks: map }
    }

    pub fn import_snapshot(&self, snapshot: CloudFormationStateSnapshot) {
        self.stacks.clear();
        for (name, stack) in snapshot.stacks {
            let stack_id = stack.stack_id.clone();
            let arc = Arc::new(RwLock::new(stack));
            self.stacks.insert(name, arc.clone());
            self.stacks.insert(stack_id, arc);
        }
    }

    pub fn reset(&self) {
        self.stacks.clear();
    }
}
