use crate::state::MskState;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_msk_request(
    State(state): State<MskState>,
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

    let is_bootstrap = parts.last() == Some(&"bootstrap-brokers");

    let action = if let Some(t) = target {
        t
    } else if parts.first() == Some(&"v1") && parts.get(1) == Some(&"clusters") {
        if is_bootstrap {
            "GetBootstrapBrokers".to_string()
        } else if parts.len() == 2 {
            match method {
                Method::POST => "CreateCluster".to_string(),
                Method::GET => "ListClusters".to_string(),
                _ => "Unknown".to_string(),
            }
        } else {
            match method {
                Method::GET => "DescribeCluster".to_string(),
                Method::DELETE => "DeleteCluster".to_string(),
                _ => "Unknown".to_string(),
            }
        }
    } else {
        "Unknown".to_string()
    };

    let cluster_id_from_path =
        if parts.first() == Some(&"v1") && parts.get(1) == Some(&"clusters") && parts.len() >= 3 {
            Some(
                parts[2..if is_bootstrap {
                    parts.len() - 1
                } else {
                    parts.len()
                }]
                    .join("/"),
            )
        } else {
            None
        };

    match action.as_str() {
        "CreateCluster" => {
            let cluster_name = val["ClusterName"].as_str().unwrap_or_default().to_string();
            let broker_nodes = val["NumberOfBrokerNodes"].as_i64().unwrap_or(3) as i32;
            let kafka_version = val["KafkaVersion"].as_str().map(String::from);
            let tags = val.get("Tags").and_then(|t| {
                serde_json::from_value::<std::collections::HashMap<String, String>>(t.clone()).ok()
            });

            match state.create_cluster(cluster_name, broker_nodes, kafka_version, tags) {
                Ok(cluster) => json_response(
                    StatusCode::OK,
                    json!({
                        "ClusterArn": cluster.cluster_arn,
                        "ClusterName": cluster.cluster_name,
                        "State": cluster.state
                    }),
                ),
                Err(e) => {
                    error_response(StatusCode::BAD_REQUEST, "ConflictException", &e.to_string())
                }
            }
        }
        "DescribeCluster" => {
            let id = cluster_id_from_path
                .or_else(|| val["ClusterArn"].as_str().map(String::from))
                .unwrap_or_default();
            match state.describe_cluster(&id) {
                Ok(cluster) => json_response(StatusCode::OK, json!({ "ClusterInfo": cluster })),
                Err(e) => {
                    error_response(StatusCode::NOT_FOUND, "NotFoundException", &e.to_string())
                }
            }
        }
        "ListClusters" => {
            let list = state.list_clusters();
            json_response(StatusCode::OK, json!({ "ClusterInfoList": list }))
        }
        "DeleteCluster" => {
            let id = cluster_id_from_path
                .or_else(|| val["ClusterArn"].as_str().map(String::from))
                .unwrap_or_default();
            match state.delete_cluster(&id) {
                Ok(cluster) => json_response(
                    StatusCode::OK,
                    json!({
                        "ClusterArn": cluster.cluster_arn,
                        "State": "DELETING"
                    }),
                ),
                Err(e) => {
                    error_response(StatusCode::NOT_FOUND, "NotFoundException", &e.to_string())
                }
            }
        }
        "GetBootstrapBrokers" => {
            let id = cluster_id_from_path
                .or_else(|| val["ClusterArn"].as_str().map(String::from))
                .unwrap_or_default();
            match state.get_bootstrap_brokers(&id) {
                Ok(brokers) => json_response(StatusCode::OK, json!(brokers)),
                Err(e) => {
                    error_response(StatusCode::NOT_FOUND, "NotFoundException", &e.to_string())
                }
            }
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("Action '{}' is not supported by Amazon MSK", action),
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
