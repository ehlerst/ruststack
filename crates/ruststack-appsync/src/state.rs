use crate::types::{ApiKey, AppSyncError, DataSource, GraphqlApi};
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSyncSnapshot {
    pub apis: Vec<GraphqlApi>,
}

#[derive(Clone)]
pub struct AppSyncState {
    pub account_id: String,
    pub region: String,
    apis: Arc<DashMap<String, GraphqlApi>>,
    api_keys: Arc<DashMap<String, Vec<ApiKey>>>,
    schemas: Arc<DashMap<String, String>>,
    data_sources: Arc<DashMap<String, Vec<DataSource>>>,
}

impl AppSyncState {
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            account_id,
            region,
            apis: Arc::new(DashMap::new()),
            api_keys: Arc::new(DashMap::new()),
            schemas: Arc::new(DashMap::new()),
            data_sources: Arc::new(DashMap::new()),
        }
    }

    pub fn export_snapshot(&self) -> AppSyncSnapshot {
        AppSyncSnapshot {
            apis: self.apis.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: AppSyncSnapshot) {
        self.apis.clear();
        self.api_keys.clear();
        self.schemas.clear();
        self.data_sources.clear();
        for api in snapshot.apis {
            self.apis.insert(api.api_id.clone(), api);
        }
    }

    pub fn reset(&self) {
        self.apis.clear();
        self.api_keys.clear();
        self.schemas.clear();
        self.data_sources.clear();
    }

    pub fn create_graphql_api(
        &self,
        name: String,
        authentication_type: Option<String>,
        tags: Option<HashMap<String, String>>,
    ) -> Result<GraphqlApi, AppSyncError> {
        let api_id = Uuid::new_v4().to_string();
        let arn = format!(
            "arn:aws:appsync:{}:{}:apis/{}",
            self.region, self.account_id, api_id
        );
        let auth_type = authentication_type.unwrap_or_else(|| "API_KEY".to_string());

        let mut uris = HashMap::new();
        uris.insert(
            "GRAPHQL".to_string(),
            format!("http://localhost:4566/v1/apis/{}/graphql", api_id),
        );
        uris.insert(
            "REALTIME".to_string(),
            format!("ws://localhost:4566/v1/apis/{}/graphql", api_id),
        );

        let api = GraphqlApi {
            api_id: api_id.clone(),
            name,
            authentication_type: auth_type,
            arn,
            uris,
            tags: tags.unwrap_or_default(),
        };

        self.apis.insert(api_id, api.clone());
        Ok(api)
    }

    pub fn get_graphql_api(&self, api_id: &str) -> Result<GraphqlApi, AppSyncError> {
        self.apis
            .get(api_id)
            .map(|a| a.clone())
            .ok_or_else(|| AppSyncError::NotFound(api_id.to_string()))
    }

    pub fn list_graphql_apis(&self) -> Vec<GraphqlApi> {
        let mut list: Vec<GraphqlApi> = self.apis.iter().map(|kv| kv.value().clone()).collect();
        list.sort_by_key(|a| a.name.clone());
        list
    }

    pub fn delete_graphql_api(&self, api_id: &str) -> Result<GraphqlApi, AppSyncError> {
        self.api_keys.remove(api_id);
        self.schemas.remove(api_id);
        self.data_sources.remove(api_id);
        self.apis
            .remove(api_id)
            .map(|(_, a)| a)
            .ok_or_else(|| AppSyncError::NotFound(api_id.to_string()))
    }

    pub fn create_api_key(
        &self,
        api_id: &str,
        description: Option<String>,
        expires: Option<i64>,
    ) -> Result<ApiKey, AppSyncError> {
        self.get_graphql_api(api_id)?;
        let key_id = format!("da2-{}", Uuid::new_v4().simple());
        let now = Utc::now().timestamp();
        let exp = expires.unwrap_or(now + 604800); // 7 days

        let key = ApiKey {
            id: key_id,
            description,
            expires: exp,
            deletes: exp + 86400,
        };

        self.api_keys
            .entry(api_id.to_string())
            .or_default()
            .push(key.clone());
        Ok(key)
    }

    pub fn list_api_keys(&self, api_id: &str) -> Result<Vec<ApiKey>, AppSyncError> {
        self.get_graphql_api(api_id)?;
        Ok(self
            .api_keys
            .get(api_id)
            .map(|k| k.clone())
            .unwrap_or_default())
    }

    pub fn start_schema_creation(
        &self,
        api_id: &str,
        definition: String,
    ) -> Result<(), AppSyncError> {
        self.get_graphql_api(api_id)?;
        self.schemas.insert(api_id.to_string(), definition);
        Ok(())
    }

    pub fn get_schema_creation_status(&self, api_id: &str) -> Result<String, AppSyncError> {
        self.get_graphql_api(api_id)?;
        if self.schemas.contains_key(api_id) {
            Ok("SUCCESS".to_string())
        } else {
            Ok("NOT_APPLICABLE".to_string())
        }
    }

    pub fn get_introspection_schema(&self, api_id: &str) -> Result<String, AppSyncError> {
        self.get_graphql_api(api_id)?;
        Ok(self
            .schemas
            .get(api_id)
            .map(|s| s.clone())
            .unwrap_or_else(|| "type Query { hello: String }".to_string()))
    }

    pub fn create_data_source(
        &self,
        api_id: &str,
        name: String,
        data_source_type: String,
        description: Option<String>,
    ) -> Result<DataSource, AppSyncError> {
        self.get_graphql_api(api_id)?;
        let ds_arn = format!(
            "arn:aws:appsync:{}:{}:apis/{}/datasources/{}",
            self.region, self.account_id, api_id, name
        );

        let ds = DataSource {
            name,
            description,
            data_source_type,
            data_source_arn: ds_arn,
        };

        self.data_sources
            .entry(api_id.to_string())
            .or_default()
            .push(ds.clone());
        Ok(ds)
    }

    pub fn list_data_sources(&self, api_id: &str) -> Result<Vec<DataSource>, AppSyncError> {
        self.get_graphql_api(api_id)?;
        Ok(self
            .data_sources
            .get(api_id)
            .map(|d| d.clone())
            .unwrap_or_default())
    }
}
