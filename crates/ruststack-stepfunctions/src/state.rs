use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use ruststack_lambda::LambdaState;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum StepFunctionsError {
    #[error("StateMachineDoesNotExist: {0}")]
    StateMachineDoesNotExist(String),
    #[error("StateMachineAlreadyExists: {0}")]
    StateMachineAlreadyExists(String),
    #[error("ExecutionDoesNotExist: {0}")]
    ExecutionDoesNotExist(String),
    #[error("ExecutionAlreadyExists: {0}")]
    ExecutionAlreadyExists(String),
    #[error("InvalidDefinition: {0}")]
    InvalidDefinition(String),
    #[error("InvalidExecutionInput: {0}")]
    InvalidExecutionInput(String),
}

#[derive(Clone)]
pub struct StepFunctionsState {
    pub account_id: String,
    pub region: String,
    state_machines: Arc<DashMap<String, Arc<RwLock<StoredStateMachine>>>>,
    executions: Arc<DashMap<String, Arc<RwLock<StoredExecution>>>>,
    lambda_state: Arc<RwLock<Option<Arc<LambdaState>>>>,
}

impl StepFunctionsState {
    pub fn new(account_id: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
            region: region.into(),
            state_machines: Arc::new(DashMap::new()),
            executions: Arc::new(DashMap::new()),
            lambda_state: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_lambda_state(&self, lambda: Arc<LambdaState>) {
        *self.lambda_state.write() = Some(lambda);
    }

    pub fn format_state_machine_arn(&self, name: &str) -> String {
        format!(
            "arn:aws:states:{}:{}:stateMachine:{}",
            self.region, self.account_id, name
        )
    }

    pub fn format_execution_arn(&self, sm_name: &str, exec_name: &str) -> String {
        format!(
            "arn:aws:states:{}:{}:execution:{}:{}",
            self.region, self.account_id, sm_name, exec_name
        )
    }

    pub fn create_state_machine(
        &self,
        req: CreateStateMachineRequest,
    ) -> Result<CreateStateMachineResponse, StepFunctionsError> {
        let arn = self.format_state_machine_arn(&req.name);
        if self.state_machines.contains_key(&arn) {
            return Err(StepFunctionsError::StateMachineAlreadyExists(format!(
                "State machine {} already exists",
                arn
            )));
        }

        // Validate JSON definition
        let _: serde_json::Value = serde_json::from_str(&req.definition).map_err(|e| {
            StepFunctionsError::InvalidDefinition(format!("Invalid ASL JSON definition: {}", e))
        })?;

        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        let stored = StoredStateMachine {
            arn: arn.clone(),
            name: req.name,
            definition: req.definition,
            role_arn: req.role_arn,
            state_machine_type: req.state_machine_type.unwrap_or(StateMachineType::Standard),
            status: StateMachineStatus::Active,
            created_at: now,
        };

        self.state_machines
            .insert(arn.clone(), Arc::new(RwLock::new(stored)));

        Ok(CreateStateMachineResponse {
            state_machine_arn: arn,
            creation_date: now,
        })
    }

    pub fn describe_state_machine(
        &self,
        req: DescribeStateMachineRequest,
    ) -> Result<DescribeStateMachineResponse, StepFunctionsError> {
        let sm_entry = self.state_machines.get(&req.state_machine_arn).ok_or_else(|| {
            StepFunctionsError::StateMachineDoesNotExist(format!(
                "State Machine {} not found",
                req.state_machine_arn
            ))
        })?;

        let sm = sm_entry.read();
        Ok(DescribeStateMachineResponse {
            state_machine_arn: sm.arn.clone(),
            name: sm.name.clone(),
            status: sm.status.clone(),
            definition: sm.definition.clone(),
            role_arn: sm.role_arn.clone(),
            state_machine_type: sm.state_machine_type.clone(),
            creation_date: sm.created_at,
        })
    }

    pub fn list_state_machines(
        &self,
        req: ListStateMachinesRequest,
    ) -> Result<ListStateMachinesResponse, StepFunctionsError> {
        let mut list = Vec::new();
        for item in self.state_machines.iter() {
            let sm = item.value().read();
            list.push(StateMachineListItem {
                state_machine_arn: sm.arn.clone(),
                name: sm.name.clone(),
                state_machine_type: sm.state_machine_type.clone(),
                creation_date: sm.created_at,
            });
        }
        list.sort_by(|a, b| a.name.cmp(&b.name));
        let limit = req.max_results.unwrap_or(100);
        if list.len() > limit {
            list.truncate(limit);
        }

        Ok(ListStateMachinesResponse {
            state_machines: list,
            next_token: None,
        })
    }

    pub fn delete_state_machine(
        &self,
        req: DeleteStateMachineRequest,
    ) -> Result<(), StepFunctionsError> {
        self.state_machines
            .remove(&req.state_machine_arn)
            .ok_or_else(|| {
                StepFunctionsError::StateMachineDoesNotExist(format!(
                    "State Machine {} not found",
                    req.state_machine_arn
                ))
            })?;
        Ok(())
    }

    pub fn start_execution(
        &self,
        req: StartExecutionRequest,
    ) -> Result<StartExecutionResponse, StepFunctionsError> {
        let sm_entry = self.state_machines.get(&req.state_machine_arn).ok_or_else(|| {
            StepFunctionsError::StateMachineDoesNotExist(format!(
                "State Machine {} not found",
                req.state_machine_arn
            ))
        })?;

        let sm = sm_entry.read().clone();
        let exec_name = req
            .name
            .unwrap_or_else(|| format!("{:x}", Uuid::new_v4().as_u128())[..12].to_string());
        let exec_arn = self.format_execution_arn(&sm.name, &exec_name);

        if self.executions.contains_key(&exec_arn) {
            return Err(StepFunctionsError::ExecutionAlreadyExists(format!(
                "Execution {} already exists",
                exec_arn
            )));
        }

        let input_str = req.input.unwrap_or_else(|| "{}".to_string());
        let input_val: serde_json::Value = serde_json::from_str(&input_str).map_err(|e| {
            StepFunctionsError::InvalidExecutionInput(format!("Invalid JSON input: {}", e))
        })?;

        let start_time = Utc::now().timestamp_millis() as f64 / 1000.0;
        let mut events = Vec::new();
        let mut event_id = 1;

        events.push(HistoryEvent {
            timestamp: start_time,
            event_type: "ExecutionStarted".to_string(),
            id: event_id,
            previous_event_id: 0,
            execution_started_event_details: Some(serde_json::json!({
                "input": input_str,
                "roleArn": sm.role_arn
            })),
            execution_succeeded_event_details: None,
            execution_failed_event_details: None,
            state_entered_event_details: None,
            state_exited_event_details: None,
        });

        // Run ASL interpreter engine
        let execution_result = self.execute_asl(&sm.definition, input_val, &mut events, &mut event_id);

        let stop_time = Utc::now().timestamp_millis() as f64 / 1000.0;
        let (status, output, error, cause) = match execution_result {
            Ok(out) => {
                event_id += 1;
                events.push(HistoryEvent {
                    timestamp: stop_time,
                    event_type: "ExecutionSucceeded".to_string(),
                    id: event_id,
                    previous_event_id: event_id - 1,
                    execution_started_event_details: None,
                    execution_succeeded_event_details: Some(serde_json::json!({
                        "output": out.to_string()
                    })),
                    execution_failed_event_details: None,
                    state_entered_event_details: None,
                    state_exited_event_details: None,
                });
                (ExecutionStatus::Succeeded, Some(out.to_string()), None, None)
            }
            Err(e) => {
                event_id += 1;
                events.push(HistoryEvent {
                    timestamp: stop_time,
                    event_type: "ExecutionFailed".to_string(),
                    id: event_id,
                    previous_event_id: event_id - 1,
                    execution_started_event_details: None,
                    execution_succeeded_event_details: None,
                    execution_failed_event_details: Some(serde_json::json!({
                        "error": "ExecutionError",
                        "cause": e.clone()
                    })),
                    state_entered_event_details: None,
                    state_exited_event_details: None,
                });
                (ExecutionStatus::Failed, None, Some("ExecutionError".to_string()), Some(e))
            }
        };

        let stored_exec = StoredExecution {
            execution_arn: exec_arn.clone(),
            state_machine_arn: sm.arn,
            name: exec_name,
            status,
            start_date: start_time,
            stop_date: Some(stop_time),
            input: Some(input_str),
            output,
            error,
            cause,
            events,
        };

        self.executions
            .insert(exec_arn.clone(), Arc::new(RwLock::new(stored_exec)));

        Ok(StartExecutionResponse {
            execution_arn: exec_arn,
            start_date: start_time,
        })
    }

    fn execute_asl(
        &self,
        def_str: &str,
        initial_input: serde_json::Value,
        events: &mut Vec<HistoryEvent>,
        event_id: &mut i64,
    ) -> Result<serde_json::Value, String> {
        let def: serde_json::Value =
            serde_json::from_str(def_str).map_err(|e| format!("ASL parse error: {}", e))?;

        let start_at = def
            .get("StartAt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing StartAt in ASL".to_string())?;

        let states = def
            .get("States")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "Missing States in ASL".to_string())?;

        let mut current_state_name = start_at.to_string();
        let mut current_data = initial_input;

        let mut step_count = 0;
        while step_count < 1000 {
            step_count += 1;
            let state_def = states
                .get(&current_state_name)
                .ok_or_else(|| format!("State {} not defined in ASL", current_state_name))?;

            let state_type = state_def
                .get("Type")
                .and_then(|v| v.as_str())
                .unwrap_or("Pass");

            *event_id += 1;
            let now = Utc::now().timestamp_millis() as f64 / 1000.0;
            events.push(HistoryEvent {
                timestamp: now,
                event_type: format!("{}StateEntered", state_type),
                id: *event_id,
                previous_event_id: *event_id - 1,
                execution_started_event_details: None,
                execution_succeeded_event_details: None,
                execution_failed_event_details: None,
                state_entered_event_details: Some(serde_json::json!({
                    "name": current_state_name,
                    "input": current_data.to_string()
                })),
                state_exited_event_details: None,
            });

            match state_type {
                "Pass" => {
                    if let Some(res) = state_def.get("Result") {
                        current_data = res.clone();
                    }
                }
                "Task" => {
                    let resource = state_def
                        .get("Resource")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if resource.contains(":lambda:") || resource.starts_with("arn:aws:lambda:") {
                        let lambda_opt = self.lambda_state.read().clone();
                        if let Some(lambda) = lambda_opt {
                            let payload = serde_json::to_vec(&current_data).unwrap_or_default();
                            let inv = lambda
                                .invoke_function(
                                    resource,
                                    Some(payload),
                                    Some(ruststack_lambda::types::InvocationType::RequestResponse),
                                )
                                .map_err(|e| format!("Lambda task error: {}", e))?;

                            if let Ok(res_val) = serde_json::from_slice::<serde_json::Value>(&inv.payload) {
                                current_data = res_val;
                            }
                        }
                    }
                }
                "Choice" => {
                    let choices = state_def.get("Choices").and_then(|v| v.as_array());
                    let mut matched_next: Option<String> = None;

                    if let Some(rules) = choices {
                        for rule in rules {
                            if let Some(var_path) = rule.get("Variable").and_then(|v| v.as_str()) {
                                let key = var_path.trim_start_matches("$.");
                                let var_val = current_data.get(key);

                                let is_match = if let Some(target_str) = rule.get("StringEquals").and_then(|v| v.as_str()) {
                                    var_val.and_then(|v| v.as_str()) == Some(target_str)
                                } else if let Some(target_num) = rule.get("NumericEquals").and_then(|v| v.as_f64()) {
                                    var_val.and_then(|v| v.as_f64()) == Some(target_num)
                                } else if let Some(target_bool) = rule.get("BooleanEquals").and_then(|v| v.as_bool()) {
                                    var_val.and_then(|v| v.as_bool()) == Some(target_bool)
                                } else {
                                    false
                                };

                                if is_match {
                                    matched_next = rule.get("Next").and_then(|v| v.as_str()).map(String::from);
                                    break;
                                }
                            }
                        }
                    }

                    if matched_next.is_none() {
                        matched_next = state_def.get("Default").and_then(|v| v.as_str()).map(String::from);
                    }

                    if let Some(next_state) = matched_next {
                        current_state_name = next_state;
                        continue;
                    } else {
                        return Err(format!("Choice state {} had no matching rule or Default", current_state_name));
                    }
                }
                "Wait" => {
                    // Fast in-memory execution: minimal delay
                }
                "Succeed" => {
                    *event_id += 1;
                    events.push(HistoryEvent {
                        timestamp: now,
                        event_type: "SucceedStateExited".to_string(),
                        id: *event_id,
                        previous_event_id: *event_id - 1,
                        execution_started_event_details: None,
                        execution_succeeded_event_details: None,
                        execution_failed_event_details: None,
                        state_entered_event_details: None,
                        state_exited_event_details: Some(serde_json::json!({
                            "name": current_state_name,
                            "output": current_data.to_string()
                        })),
                    });
                    return Ok(current_data);
                }
                "Fail" => {
                    let err = state_def.get("Error").and_then(|v| v.as_str()).unwrap_or("FailState");
                    let cause = state_def.get("Cause").and_then(|v| v.as_str()).unwrap_or("FailState execution");
                    return Err(format!("{}: {}", err, cause));
                }
                _ => {}
            }

            *event_id += 1;
            events.push(HistoryEvent {
                timestamp: now,
                event_type: format!("{}StateExited", state_type),
                id: *event_id,
                previous_event_id: *event_id - 1,
                execution_started_event_details: None,
                execution_succeeded_event_details: None,
                execution_failed_event_details: None,
                state_entered_event_details: None,
                state_exited_event_details: Some(serde_json::json!({
                    "name": current_state_name,
                    "output": current_data.to_string()
                })),
            });

            if state_def.get("End").and_then(|v| v.as_bool()).unwrap_or(false) {
                return Ok(current_data);
            }

            if let Some(next) = state_def.get("Next").and_then(|v| v.as_str()) {
                current_state_name = next.to_string();
            } else {
                return Ok(current_data);
            }
        }

        Err("StepFunctions execution iteration limit exceeded".to_string())
    }

    pub fn describe_execution(
        &self,
        req: DescribeExecutionRequest,
    ) -> Result<DescribeExecutionResponse, StepFunctionsError> {
        let exec_entry = self.executions.get(&req.execution_arn).ok_or_else(|| {
            StepFunctionsError::ExecutionDoesNotExist(format!(
                "Execution {} not found",
                req.execution_arn
            ))
        })?;

        let exec = exec_entry.read();
        Ok(DescribeExecutionResponse {
            execution_arn: exec.execution_arn.clone(),
            state_machine_arn: exec.state_machine_arn.clone(),
            name: exec.name.clone(),
            status: exec.status.clone(),
            start_date: exec.start_date,
            stop_date: exec.stop_date,
            input: exec.input.clone(),
            output: exec.output.clone(),
            error: exec.error.clone(),
            cause: exec.cause.clone(),
        })
    }

    pub fn get_execution_history(
        &self,
        req: GetExecutionHistoryRequest,
    ) -> Result<GetExecutionHistoryResponse, StepFunctionsError> {
        let exec_entry = self.executions.get(&req.execution_arn).ok_or_else(|| {
            StepFunctionsError::ExecutionDoesNotExist(format!(
                "Execution {} not found",
                req.execution_arn
            ))
        })?;

        let exec = exec_entry.read();
        let mut events = exec.events.clone();
        if req.reverse_order.unwrap_or(false) {
            events.reverse();
        }
        let limit = req.max_results.unwrap_or(100);
        if events.len() > limit {
            events.truncate(limit);
        }

        Ok(GetExecutionHistoryResponse {
            events,
            next_token: None,
        })
    }

    pub fn list_executions(
        &self,
        req: ListExecutionsRequest,
    ) -> Result<ListExecutionsResponse, StepFunctionsError> {
        let mut list = Vec::new();
        for item in self.executions.iter() {
            let exec = item.value().read();
            if exec.state_machine_arn == req.state_machine_arn {
                if let Some(ref filter) = req.status_filter {
                    if &exec.status != filter {
                        continue;
                    }
                }
                list.push(ExecutionListItem {
                    execution_arn: exec.execution_arn.clone(),
                    state_machine_arn: exec.state_machine_arn.clone(),
                    name: exec.name.clone(),
                    status: exec.status.clone(),
                    start_date: exec.start_date,
                    stop_date: exec.stop_date,
                });
            }
        }
        list.sort_by(|a, b| b.start_date.partial_cmp(&a.start_date).unwrap());
        let limit = req.max_results.unwrap_or(100);
        if list.len() > limit {
            list.truncate(limit);
        }

        Ok(ListExecutionsResponse {
            executions: list,
            next_token: None,
        })
    }

    pub fn export_snapshot(&self) -> StepFunctionsStateSnapshot {
        let mut sm_map = HashMap::new();
        for item in self.state_machines.iter() {
            sm_map.insert(item.key().clone(), item.value().read().clone());
        }
        let mut exec_map = HashMap::new();
        for item in self.executions.iter() {
            exec_map.insert(item.key().clone(), item.value().read().clone());
        }
        StepFunctionsStateSnapshot {
            state_machines: sm_map,
            executions: exec_map,
        }
    }

    pub fn import_snapshot(&self, snapshot: StepFunctionsStateSnapshot) {
        self.state_machines.clear();
        self.executions.clear();
        for (k, v) in snapshot.state_machines {
            self.state_machines.insert(k, Arc::new(RwLock::new(v)));
        }
        for (k, v) in snapshot.executions {
            self.executions.insert(k, Arc::new(RwLock::new(v)));
        }
    }

    pub fn reset(&self) {
        self.state_machines.clear();
        self.executions.clear();
    }
}
