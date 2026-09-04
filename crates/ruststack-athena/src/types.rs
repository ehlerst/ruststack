use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct QueryExecutionContext {
    pub database: Option<String>,
    pub catalog: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResultConfiguration {
    pub output_location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct QueryExecutionStatus {
    pub state: String,
    pub submission_date_time: DateTime<Utc>,
    pub completion_date_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct QueryExecutionStatistics {
    pub engine_execution_time_in_millis: i64,
    pub data_scanned_in_bytes: i64,
    pub total_execution_time_in_millis: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct QueryExecution {
    pub query_execution_id: String,
    pub query: String,
    pub statement_type: String,
    pub result_configuration: ResultConfiguration,
    pub query_execution_context: Option<QueryExecutionContext>,
    pub status: QueryExecutionStatus,
    pub statistics: QueryExecutionStatistics,
    pub work_group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StartQueryExecutionRequest {
    pub query_string: String,
    pub query_execution_context: Option<QueryExecutionContext>,
    pub result_configuration: Option<ResultConfiguration>,
    pub work_group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StartQueryExecutionResponse {
    pub query_execution_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetQueryExecutionRequest {
    pub query_execution_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetQueryExecutionResponse {
    pub query_execution: QueryExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ColumnInfo {
    pub name: String,
    #[serde(rename = "Type")]
    pub col_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResultSetMetadata {
    pub column_info: Vec<ColumnInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Datum {
    pub var_char_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Row {
    pub data: Vec<Datum>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResultSet {
    pub rows: Vec<Row>,
    pub result_set_metadata: ResultSetMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetQueryResultsResponse {
    pub result_set: ResultSet,
    pub update_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NamedQuery {
    pub named_query_id: String,
    pub name: String,
    pub description: Option<String>,
    pub database: String,
    pub query_string: String,
    pub work_group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateNamedQueryRequest {
    pub name: String,
    pub description: Option<String>,
    pub database: String,
    pub query_string: String,
    pub work_group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateNamedQueryResponse {
    pub named_query_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListNamedQueriesResponse {
    pub named_query_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WorkGroupSummary {
    pub name: String,
    pub state: String,
    pub description: Option<String>,
    pub creation_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListWorkGroupsResponse {
    pub work_groups: Vec<WorkGroupSummary>,
}
