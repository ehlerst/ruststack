use crate::types::{Pipe, PipeSummary, PipesError};
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipesSnapshot {
    pub pipes: Vec<Pipe>,
}

#[derive(Clone)]
pub struct PipesState {
    pub account_id: String,
    pub region: String,
    pipes: Arc<DashMap<String, Pipe>>,
}

impl PipesState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            pipes: Arc::new(DashMap::new()),
        }
    }

    pub fn export_snapshot(&self) -> PipesSnapshot {
        PipesSnapshot {
            pipes: self.pipes.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: PipesSnapshot) {
        self.pipes.clear();
        for pipe in snapshot.pipes {
            self.pipes.insert(pipe.name.clone(), pipe);
        }
    }

    pub fn reset(&self) {
        self.pipes.clear();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_pipe(
        &self,
        name: String,
        source: String,
        target: String,
        role_arn: String,
        desired_state: Option<String>,
        enrichment: Option<String>,
        description: Option<String>,
        source_parameters: Option<serde_json::Value>,
        target_parameters: Option<serde_json::Value>,
        tags: Option<HashMap<String, String>>,
    ) -> Result<Pipe, PipesError> {
        if self.pipes.contains_key(&name) {
            return Err(PipesError::PipeAlreadyExists(name));
        }

        let arn = format!(
            "arn:aws:pipes:{}:{}:pipe/{}",
            self.region, self.account_id, name
        );
        let now = Utc::now().to_rfc3339();
        let state_val = desired_state.unwrap_or_else(|| "RUNNING".to_string());

        let pipe = Pipe {
            name: name.clone(),
            arn,
            desired_state: state_val.clone(),
            current_state: state_val,
            source,
            target,
            enrichment,
            role_arn,
            description,
            source_parameters,
            target_parameters,
            creation_time: now.clone(),
            last_modified_time: now,
            tags: tags.unwrap_or_default(),
        };

        self.pipes.insert(name, pipe.clone());
        Ok(pipe)
    }

    pub fn describe_pipe(&self, name: &str) -> Result<Pipe, PipesError> {
        self.pipes
            .get(name)
            .map(|p| p.clone())
            .ok_or_else(|| PipesError::PipeNotFound(name.to_string()))
    }

    pub fn list_pipes(&self) -> Vec<PipeSummary> {
        let mut list: Vec<PipeSummary> = self
            .pipes
            .iter()
            .map(|kv| {
                let p = kv.value();
                PipeSummary {
                    name: p.name.clone(),
                    arn: p.arn.clone(),
                    desired_state: p.desired_state.clone(),
                    current_state: p.current_state.clone(),
                    source: p.source.clone(),
                    target: p.target.clone(),
                    creation_time: p.creation_time.clone(),
                    last_modified_time: p.last_modified_time.clone(),
                }
            })
            .collect();

        list.sort_by_key(|a| a.name.clone());
        list
    }

    pub fn delete_pipe(&self, name: &str) -> Result<Pipe, PipesError> {
        self.pipes
            .remove(name)
            .map(|(_, p)| p)
            .ok_or_else(|| PipesError::PipeNotFound(name.to_string()))
    }

    pub fn start_pipe(&self, name: &str) -> Result<Pipe, PipesError> {
        let mut pipe = self
            .pipes
            .get_mut(name)
            .ok_or_else(|| PipesError::PipeNotFound(name.to_string()))?;
        pipe.desired_state = "RUNNING".to_string();
        pipe.current_state = "RUNNING".to_string();
        pipe.last_modified_time = Utc::now().to_rfc3339();
        Ok(pipe.clone())
    }

    pub fn stop_pipe(&self, name: &str) -> Result<Pipe, PipesError> {
        let mut pipe = self
            .pipes
            .get_mut(name)
            .ok_or_else(|| PipesError::PipeNotFound(name.to_string()))?;
        pipe.desired_state = "STOPPED".to_string();
        pipe.current_state = "STOPPED".to_string();
        pipe.last_modified_time = Utc::now().to_rfc3339();
        Ok(pipe.clone())
    }
}
