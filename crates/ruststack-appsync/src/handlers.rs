use crate::state::AppSyncState;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_appsync_request(
    State(state): State<AppSyncState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .map(|t| t.split('.').next_back().unwrap_or(t).to_string());

    let val: serde_json::Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let path = uri.path();
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();

    // Check action from target or REST path
    let action = if let Some(t) = target {
        t
    } else if parts.first() == Some(&"v1") && parts.get(1) == Some(&"apis") {
        if parts.len() == 2 {
            match method {
                Method::POST => "CreateGraphqlApi".to_string(),
                Method::GET => "ListGraphqlApis".to_string(),
                _ => "Unknown".to_string(),
            }
        } else if parts.len() == 3 {
            match method {
                Method::GET => "GetGraphqlApi".to_string(),
                Method::DELETE => "DeleteGraphqlApi".to_string(),
                _ => "Unknown".to_string(),
            }
        } else if parts.len() == 4 {
            match (parts.get(3), method) {
                (Some(&"apikeys"), Method::POST) => "CreateApiKey".to_string(),
                (Some(&"apikeys"), Method::GET) => "ListApiKeys".to_string(),
                (Some(&"schemacreation"), Method::POST) => "StartSchemaCreation".to_string(),
                (Some(&"schemacreation"), Method::GET) => "GetSchemaCreationStatus".to_string(),
                (Some(&"datasources"), Method::POST) => "CreateDataSource".to_string(),
                (Some(&"datasources"), Method::GET) => "ListDataSources".to_string(),
                (Some(&"graphql"), Method::POST) => "ExecuteGraphQL".to_string(),
                _ => "Unknown".to_string(),
            }
        } else {
            "Unknown".to_string()
        }
    } else {
        "Unknown".to_string()
    };

    let api_id_from_path =
        if parts.first() == Some(&"v1") && parts.get(1) == Some(&"apis") && parts.len() >= 3 {
            Some(parts[2].to_string())
        } else {
            None
        };

    match action.as_str() {
        "CreateGraphqlApi" => {
            let name = val["name"].as_str().unwrap_or_default().to_string();
            let auth_type = val["authenticationType"].as_str().map(String::from);
            let tags = val.get("tags").and_then(|t| {
                serde_json::from_value::<std::collections::HashMap<String, String>>(t.clone()).ok()
            });

            match state.create_graphql_api(name, auth_type, tags) {
                Ok(api) => json_response(StatusCode::OK, json!({ "graphqlApi": api })),
                Err(e) => error_response(
                    StatusCode::BAD_REQUEST,
                    "BadRequestException",
                    &e.to_string(),
                ),
            }
        }
        "GetGraphqlApi" => {
            let id = api_id_from_path
                .or_else(|| val["apiId"].as_str().map(String::from))
                .unwrap_or_default();
            match state.get_graphql_api(&id) {
                Ok(api) => json_response(StatusCode::OK, json!({ "graphqlApi": api })),
                Err(e) => {
                    error_response(StatusCode::NOT_FOUND, "NotFoundException", &e.to_string())
                }
            }
        }
        "ListGraphqlApis" => {
            let list = state.list_graphql_apis();
            json_response(StatusCode::OK, json!({ "graphqlApis": list }))
        }
        "DeleteGraphqlApi" => {
            let id = api_id_from_path
                .or_else(|| val["apiId"].as_str().map(String::from))
                .unwrap_or_default();
            match state.delete_graphql_api(&id) {
                Ok(_) => json_response(StatusCode::OK, json!({})),
                Err(e) => {
                    error_response(StatusCode::NOT_FOUND, "NotFoundException", &e.to_string())
                }
            }
        }
        "CreateApiKey" => {
            let id = api_id_from_path
                .or_else(|| val["apiId"].as_str().map(String::from))
                .unwrap_or_default();
            let description = val["description"].as_str().map(String::from);
            let expires = val["expires"].as_i64();
            match state.create_api_key(&id, description, expires) {
                Ok(key) => json_response(StatusCode::OK, json!({ "apiKey": key })),
                Err(e) => error_response(
                    StatusCode::BAD_REQUEST,
                    "BadRequestException",
                    &e.to_string(),
                ),
            }
        }
        "ListApiKeys" => {
            let id = api_id_from_path
                .or_else(|| val["apiId"].as_str().map(String::from))
                .unwrap_or_default();
            match state.list_api_keys(&id) {
                Ok(keys) => json_response(StatusCode::OK, json!({ "apiKeys": keys })),
                Err(e) => {
                    error_response(StatusCode::NOT_FOUND, "NotFoundException", &e.to_string())
                }
            }
        }
        "StartSchemaCreation" => {
            let id = api_id_from_path
                .or_else(|| val["apiId"].as_str().map(String::from))
                .unwrap_or_default();
            let definition = val["definition"].as_str().unwrap_or_default().to_string();
            match state.start_schema_creation(&id, definition) {
                Ok(_) => json_response(StatusCode::OK, json!({ "status": "PROCESSING" })),
                Err(e) => error_response(
                    StatusCode::BAD_REQUEST,
                    "BadRequestException",
                    &e.to_string(),
                ),
            }
        }
        "GetSchemaCreationStatus" => {
            let id = api_id_from_path
                .or_else(|| val["apiId"].as_str().map(String::from))
                .unwrap_or_default();
            match state.get_schema_creation_status(&id) {
                Ok(st) => json_response(StatusCode::OK, json!({ "status": st })),
                Err(e) => {
                    error_response(StatusCode::NOT_FOUND, "NotFoundException", &e.to_string())
                }
            }
        }
        "CreateDataSource" => {
            let id = api_id_from_path
                .or_else(|| val["apiId"].as_str().map(String::from))
                .unwrap_or_default();
            let name = val["name"].as_str().unwrap_or_default().to_string();
            let ds_type = val["type"].as_str().unwrap_or("NONE").to_string();
            let description = val["description"].as_str().map(String::from);
            match state.create_data_source(&id, name, ds_type, description) {
                Ok(ds) => json_response(StatusCode::OK, json!({ "dataSource": ds })),
                Err(e) => error_response(
                    StatusCode::BAD_REQUEST,
                    "BadRequestException",
                    &e.to_string(),
                ),
            }
        }
        "ListDataSources" => {
            let id = api_id_from_path
                .or_else(|| val["apiId"].as_str().map(String::from))
                .unwrap_or_default();
            match state.list_data_sources(&id) {
                Ok(data_sources) => {
                    json_response(StatusCode::OK, json!({ "dataSources": data_sources }))
                }
                Err(e) => {
                    error_response(StatusCode::NOT_FOUND, "NotFoundException", &e.to_string())
                }
            }
        }
        "ExecuteGraphQL" => {
            let query = val["query"].as_str().unwrap_or_default();
            // Return realistic simulated GraphQL response
            json_response(
                StatusCode::OK,
                json!({
                    "data": {
                        "hello": "world",
                        "status": "RustStack AppSync Simulated Response",
                        "query_executed": query
                    }
                }),
            )
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("Action '{}' is not supported by AWS AppSync", action),
        ),
    }
}

fn json_response(status: StatusCode, body: serde_json::Value) -> Response {
    (
        status,
        [("content-type", "application/json")],
        body.to_string(),
    )
        .into_response()
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        [("content-type", "application/json")],
        json!({
            "__type": code,
            "message": message
        })
        .to_string(),
    )
        .into_response()
}
