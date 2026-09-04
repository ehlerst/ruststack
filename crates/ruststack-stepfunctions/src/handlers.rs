use crate::state::StepFunctionsState;
use crate::types::*;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_stepfunctions_request(
    State(state): State<StepFunctionsState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let action = if let Some(pos) = target.rfind('.') {
        &target[pos + 1..]
    } else {
        target
    };

    match action {
        "CreateStateMachine" => {
            let req: CreateStateMachineRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.create_state_machine(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        "DescribeStateMachine" => {
            let req: DescribeStateMachineRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.describe_state_machine(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        "ListStateMachines" => {
            let req: ListStateMachinesRequest = serde_json::from_slice(&body).unwrap_or_default();
            match state.list_state_machines(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        "DeleteStateMachine" => {
            let req: DeleteStateMachineRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.delete_state_machine(req) {
                Ok(_) => ok_json(&json!({})),
                Err(e) => error_response(e),
            }
        }
        "StartExecution" => {
            let req: StartExecutionRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.start_execution(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        "DescribeExecution" => {
            let req: DescribeExecutionRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.describe_execution(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        "GetExecutionHistory" => {
            let req: GetExecutionHistoryRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.get_execution_history(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        "ListExecutions" => {
            let req: ListExecutionsRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return bad_request(&e.to_string()),
            };
            match state.list_executions(req) {
                Ok(resp) => ok_json(&resp),
                Err(e) => error_response(e),
            }
        }
        _ => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("content-type", "application/x-amz-json-1.0")
            .body(Body::from(
                json!({
                    "__type": "InvalidAction",
                    "message": format!("Unknown action {}", action)
                })
                .to_string(),
            ))
            .unwrap(),
    }
}

fn ok_json<T: serde::Serialize>(val: &T) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(serde_json::to_string(val).unwrap()))
        .unwrap()
}

fn bad_request(msg: &str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "__type": "InvalidParameterValue",
                "message": msg
            })
            .to_string(),
        ))
        .unwrap()
}

fn error_response(err: crate::state::StepFunctionsError) -> Response<Body> {
    use crate::state::StepFunctionsError::*;
    let (status, type_name, msg) = match err {
        StateMachineDoesNotExist(m) => (StatusCode::BAD_REQUEST, "StateMachineDoesNotExist", m),
        StateMachineAlreadyExists(m) => (StatusCode::BAD_REQUEST, "StateMachineAlreadyExists", m),
        ExecutionDoesNotExist(m) => (StatusCode::BAD_REQUEST, "ExecutionDoesNotExist", m),
        ExecutionAlreadyExists(m) => (StatusCode::BAD_REQUEST, "ExecutionAlreadyExists", m),
        InvalidDefinition(m) => (StatusCode::BAD_REQUEST, "InvalidDefinition", m),
        InvalidExecutionInput(m) => (StatusCode::BAD_REQUEST, "InvalidExecutionInput", m),
    };

    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "__type": type_name,
                "message": msg
            })
            .to_string(),
        ))
        .unwrap()
}
