use crate::state::AthenaState;
use crate::types::*;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_athena_request(
    State(state): State<AthenaState>,
    headers: HeaderMap,
    body_bytes: Bytes,
) -> Response<Body> {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let action = if let Some(a) = target.split('.').nth(1) {
        a
    } else {
        target
    };

    let req_json: serde_json::Value = if body_bytes.is_empty() {
        json!({})
    } else {
        match serde_json::from_slice(&body_bytes) {
            Ok(v) => v,
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("content-type", "application/x-amz-json-1.1")
                    .body(Body::from(json!({ "__type": "InvalidRequestException", "message": format!("Invalid JSON: {}", e) }).to_string()))
                    .unwrap();
            }
        }
    };

    match action {
        "StartQueryExecution" => {
            let req: StartQueryExecutionRequest = match serde_json::from_value(req_json) {
                Ok(r) => r,
                Err(e) => {
                    return Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .header("content-type", "application/x-amz-json-1.1")
                        .body(Body::from(json!({ "__type": "InvalidRequestException", "message": e.to_string() }).to_string()))
                        .unwrap();
                }
            };

            let qid = state.start_query_execution(req);
            let resp = StartQueryExecutionResponse {
                query_execution_id: qid,
            };
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(serde_json::to_string(&resp).unwrap()))
                .unwrap()
        }
        "GetQueryExecution" => {
            let req: GetQueryExecutionRequest = match serde_json::from_value(req_json) {
                Ok(r) => r,
                Err(e) => {
                    return Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .header("content-type", "application/x-amz-json-1.1")
                        .body(Body::from(json!({ "__type": "InvalidRequestException", "message": e.to_string() }).to_string()))
                        .unwrap();
                }
            };

            match state.get_query_execution(&req.query_execution_id) {
                Ok(query_execution) => {
                    let resp = GetQueryExecutionResponse { query_execution };
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "application/x-amz-json-1.1")
                        .body(Body::from(serde_json::to_string(&resp).unwrap()))
                        .unwrap()
                }
                Err(e) => Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("content-type", "application/x-amz-json-1.1")
                    .body(Body::from(json!({ "__type": "InvalidRequestException", "message": e.to_string() }).to_string()))
                    .unwrap(),
            }
        }
        "GetQueryResults" => {
            let qid = req_json.get("QueryExecutionId").and_then(|v| v.as_str()).unwrap_or("");
            match state.get_query_results(qid) {
                Ok(resp) => Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "application/x-amz-json-1.1")
                    .body(Body::from(serde_json::to_string(&resp).unwrap()))
                    .unwrap(),
                Err(e) => Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("content-type", "application/x-amz-json-1.1")
                    .body(Body::from(json!({ "__type": "InvalidRequestException", "message": e.to_string() }).to_string()))
                    .unwrap(),
            }
        }
        "CreateNamedQuery" => {
            let req: CreateNamedQueryRequest = match serde_json::from_value(req_json) {
                Ok(r) => r,
                Err(e) => {
                    return Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .header("content-type", "application/x-amz-json-1.1")
                        .body(Body::from(json!({ "__type": "InvalidRequestException", "message": e.to_string() }).to_string()))
                        .unwrap();
                }
            };

            let nid = state.create_named_query(req);
            let resp = CreateNamedQueryResponse { named_query_id: nid };
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(serde_json::to_string(&resp).unwrap()))
                .unwrap()
        }
        "ListNamedQueries" => {
            let ids = state.list_named_queries();
            let resp = ListNamedQueriesResponse { named_query_ids: ids };
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(serde_json::to_string(&resp).unwrap()))
                .unwrap()
        }
        "ListWorkGroups" => {
            let wgs = state.list_work_groups();
            let resp = ListWorkGroupsResponse { work_groups: wgs };
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(serde_json::to_string(&resp).unwrap()))
                .unwrap()
        }
        _ => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("content-type", "application/x-amz-json-1.1")
            .body(Body::from(json!({ "__type": "InvalidAction", "message": format!("Unknown Athena action: {}", action) }).to_string()))
            .unwrap(),
    }
}
