use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum AthenaError {
    #[error("InvalidRequestException: QueryExecutionId {0} not found")]
    QueryNotFound(String),
    #[error("InvalidRequestException: NamedQueryId {0} not found")]
    NamedQueryNotFound(String),
    #[error("InvalidRequestException: {0}")]
    InvalidRequest(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AthenaStateSnapshot {
    pub queries: Vec<QueryExecution>,
    pub named_queries: Vec<NamedQuery>,
}

#[derive(Clone)]
pub struct AthenaState {
    account_id: String,
    region: String,
    queries: Arc<DashMap<String, QueryExecution>>,
    named_queries: Arc<DashMap<String, NamedQuery>>,
    work_groups: Arc<DashMap<String, WorkGroupSummary>>,
}

impl AthenaState {
    pub fn new(account_id: String, region: String) -> Self {
        let state = Self {
            account_id,
            region,
            queries: Arc::new(DashMap::new()),
            named_queries: Arc::new(DashMap::new()),
            work_groups: Arc::new(DashMap::new()),
        };

        state.init_default_workgroup();
        state
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    fn init_default_workgroup(&self) {
        self.work_groups.insert(
            "primary".to_string(),
            WorkGroupSummary {
                name: "primary".to_string(),
                state: "ENABLED".to_string(),
                description: Some("Primary workgroup for ad-hoc queries".to_string()),
                creation_time: Utc::now(),
            },
        );
    }

    pub fn start_query_execution(&self, req: StartQueryExecutionRequest) -> String {
        let qid = Uuid::new_v4().to_string();
        let now = Utc::now();

        let output_loc = req
            .result_configuration
            .as_ref()
            .and_then(|r| r.output_location.clone())
            .unwrap_or_else(|| format!("s3://aws-athena-query-results-{}-{}/", self.account_id, self.region));

        let execution = QueryExecution {
            query_execution_id: qid.clone(),
            query: req.query_string,
            statement_type: "DML".to_string(),
            result_configuration: ResultConfiguration {
                output_location: Some(output_loc),
            },
            query_execution_context: req.query_execution_context,
            status: QueryExecutionStatus {
                state: "SUCCEEDED".to_string(),
                submission_date_time: now,
                completion_date_time: Some(now),
            },
            statistics: QueryExecutionStatistics {
                engine_execution_time_in_millis: 12,
                data_scanned_in_bytes: 1024,
                total_execution_time_in_millis: 15,
            },
            work_group: req.work_group.unwrap_or_else(|| "primary".to_string()),
        };

        self.queries.insert(qid.clone(), execution);
        qid
    }

    pub fn get_query_execution(&self, query_id: &str) -> Result<QueryExecution, AthenaError> {
        self.queries
            .get(query_id)
            .map(|kv| kv.value().clone())
            .ok_or_else(|| AthenaError::QueryNotFound(query_id.to_string()))
    }

    pub fn get_query_results(&self, query_id: &str) -> Result<GetQueryResultsResponse, AthenaError> {
        let query = self.get_query_execution(query_id)?;

        let (cols, rows) = if query.query.to_uppercase().contains("COUNT") {
            (
                vec![ColumnInfo {
                    name: "_col0".to_string(),
                    col_type: "bigint".to_string(),
                }],
                vec![
                    Row {
                        data: vec![Datum {
                            var_char_value: Some("_col0".to_string()),
                        }],
                    },
                    Row {
                        data: vec![Datum {
                            var_char_value: Some("42".to_string()),
                        }],
                    },
                ],
            )
        } else {
            (
                vec![
                    ColumnInfo {
                        name: "id".to_string(),
                        col_type: "varchar".to_string(),
                    },
                    ColumnInfo {
                        name: "value".to_string(),
                        col_type: "varchar".to_string(),
                    },
                ],
                vec![
                    Row {
                        data: vec![
                            Datum {
                                var_char_value: Some("id".to_string()),
                            },
                            Datum {
                                var_char_value: Some("value".to_string()),
                            },
                        ],
                    },
                    Row {
                        data: vec![
                            Datum {
                                var_char_value: Some("row-1".to_string()),
                            },
                            Datum {
                                var_char_value: Some("test-data".to_string()),
                            },
                        ],
                    },
                ],
            )
        };

        Ok(GetQueryResultsResponse {
            result_set: ResultSet {
                rows,
                result_set_metadata: ResultSetMetadata { column_info: cols },
            },
            update_count: 0,
        })
    }

    pub fn create_named_query(&self, req: CreateNamedQueryRequest) -> String {
        let nid = Uuid::new_v4().to_string();
        let named = NamedQuery {
            named_query_id: nid.clone(),
            name: req.name,
            description: req.description,
            database: req.database,
            query_string: req.query_string,
            work_group: req.work_group,
        };

        self.named_queries.insert(nid.clone(), named);
        nid
    }

    pub fn get_named_query(&self, named_query_id: &str) -> Result<NamedQuery, AthenaError> {
        self.named_queries
            .get(named_query_id)
            .map(|kv| kv.value().clone())
            .ok_or_else(|| AthenaError::NamedQueryNotFound(named_query_id.to_string()))
    }

    pub fn list_named_queries(&self) -> Vec<String> {
        self.named_queries.iter().map(|kv| kv.key().clone()).collect()
    }

    pub fn list_work_groups(&self) -> Vec<WorkGroupSummary> {
        self.work_groups.iter().map(|kv| kv.value().clone()).collect()
    }

    pub fn reset(&self) {
        self.queries.clear();
        self.named_queries.clear();
        self.work_groups.clear();
        self.init_default_workgroup();
    }

    pub fn export_snapshot(&self) -> AthenaStateSnapshot {
        AthenaStateSnapshot {
            queries: self.queries.iter().map(|kv| kv.value().clone()).collect(),
            named_queries: self.named_queries.iter().map(|kv| kv.value().clone()).collect(),
        }
    }

    pub fn import_snapshot(&self, snapshot: AthenaStateSnapshot) {
        self.queries.clear();
        self.named_queries.clear();
        for q in snapshot.queries {
            self.queries.insert(q.query_execution_id.clone(), q);
        }
        for n in snapshot.named_queries {
            self.named_queries.insert(n.named_query_id.clone(), n);
        }
    }
}
