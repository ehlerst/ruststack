use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode};
use bytes::Bytes;
use serde_json::json;

use crate::state::EcsState;
use crate::types::*;

pub async fn handle_ecs_request(
    State(state): State<EcsState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let action = target
        .strip_prefix("AmazonEC2ContainerServiceV20141113.")
        .unwrap_or(target);

    match action {
        "CreateCluster" => {
            let req: CreateClusterRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.create_cluster(req) {
                Ok(resp) => json_response(StatusCode::OK, &resp),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "ClientException", &e.to_string()),
            }
        }
        "DescribeClusters" => {
            let req: DescribeClustersRequest = serde_json::from_slice(&body).unwrap_or(DescribeClustersRequest { clusters: None });
            let resp = state.describe_clusters(req);
            json_response(StatusCode::OK, &resp)
        }
        "ListClusters" => {
            let resp = state.list_clusters();
            json_response(StatusCode::OK, &resp)
        }
        "DeleteCluster" => {
            let req: DeleteClusterRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.delete_cluster(req) {
                Ok(resp) => json_response(StatusCode::OK, &resp),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "ClusterNotFoundException", &e.to_string()),
            }
        }
        "RegisterTaskDefinition" => {
            let req: RegisterTaskDefinitionRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.register_task_definition(req) {
                Ok(resp) => json_response(StatusCode::OK, &resp),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "ClientException", &e.to_string()),
            }
        }
        "DescribeTaskDefinition" => {
            let req: DescribeTaskDefinitionRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.describe_task_definition(req) {
                Ok(resp) => json_response(StatusCode::OK, &resp),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "ClientException", &e.to_string()),
            }
        }
        "DeregisterTaskDefinition" => {
            let req: DeregisterTaskDefinitionRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.deregister_task_definition(req) {
                Ok(resp) => json_response(StatusCode::OK, &resp),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "ClientException", &e.to_string()),
            }
        }
        "ListTaskDefinitions" => {
            let req: ListTaskDefinitionsRequest = serde_json::from_slice(&body).unwrap_or(ListTaskDefinitionsRequest {
                family_prefix: None,
                status: None,
            });
            let resp = state.list_task_definitions(req);
            json_response(StatusCode::OK, &resp)
        }
        "RunTask" => {
            let req: RunTaskRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.run_task(req) {
                Ok(resp) => json_response(StatusCode::OK, &resp),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "ClientException", &e.to_string()),
            }
        }
        "DescribeTasks" => {
            let req: DescribeTasksRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            let resp = state.describe_tasks(req);
            json_response(StatusCode::OK, &resp)
        }
        "StopTask" => {
            let req: StopTaskRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.stop_task(req) {
                Ok(resp) => json_response(StatusCode::OK, &resp),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            }
        }
        "ListTasks" => {
            let req: ListTasksRequest = serde_json::from_slice(&body).unwrap_or(ListTasksRequest {
                cluster: None,
                family: None,
                desired_status: None,
            });
            let resp = state.list_tasks(req);
            json_response(StatusCode::OK, &resp)
        }
        "CreateService" => {
            let req: CreateServiceRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.create_service(req) {
                Ok(resp) => json_response(StatusCode::OK, &resp),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "ClusterNotFoundException", &e.to_string()),
            }
        }
        "DescribeServices" => {
            let req: DescribeServicesRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            let resp = state.describe_services(req);
            json_response(StatusCode::OK, &resp)
        }
        "UpdateService" => {
            let req: UpdateServiceRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.update_service(req) {
                Ok(resp) => json_response(StatusCode::OK, &resp),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "ServiceNotFoundException", &e.to_string()),
            }
        }
        "DeleteService" => {
            let req: DeleteServiceRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            };
            match state.delete_service(req) {
                Ok(resp) => json_response(StatusCode::OK, &resp),
                Err(e) => error_response(StatusCode::BAD_REQUEST, "ServiceNotFoundException", &e.to_string()),
            }
        }
        "ListServices" => {
            let req: ListServicesRequest = serde_json::from_slice(&body).unwrap_or(ListServicesRequest { cluster: None });
            let resp = state.list_services(req);
            json_response(StatusCode::OK, &resp)
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("The action {} is not valid for Amazon ECS.", action),
        ),
    }
}

fn json_response<T: serde::Serialize>(status: StatusCode, body: &T) -> Response<Body> {
    let bytes = serde_json::to_vec(body).unwrap_or_default();
    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(bytes))
        .unwrap()
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response<Body> {
    let err_json = json!({
        "__type": code,
        "message": message
    });
    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(err_json.to_string()))
        .unwrap()
}
