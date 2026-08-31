use crate::state::{LogsError, LogsState};
use crate::types::*;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_logs_request(
    State(state): State<LogsState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let operation = if let Some(pos) = target.find('.') {
        &target[pos + 1..]
    } else {
        target
    };

    match operation {
        "CreateLogGroup" => {
            let req: CreateLogGroupRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterException",
                        &e.to_string(),
                    )
                }
            };
            match state.create_log_group(req) {
                Ok(()) => (
                    StatusCode::OK,
                    [("content-type", "application/x-amz-json-1.1")],
                    "{}",
                )
                    .into_response(),
                Err(e) => map_logs_error(e),
            }
        }
        "DeleteLogGroup" => {
            let req: DeleteLogGroupRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterException",
                        &e.to_string(),
                    )
                }
            };
            match state.delete_log_group(req) {
                Ok(()) => (
                    StatusCode::OK,
                    [("content-type", "application/x-amz-json-1.1")],
                    "{}",
                )
                    .into_response(),
                Err(e) => map_logs_error(e),
            }
        }
        "DescribeLogGroups" => {
            let req: DescribeLogGroupsRequest = serde_json::from_slice(&body).unwrap_or_default();
            match state.describe_log_groups(req) {
                Ok((groups, next_token)) => json_response(
                    StatusCode::OK,
                    json!({
                        "logGroups": groups,
                        "nextToken": next_token
                    }),
                ),
                Err(e) => map_logs_error(e),
            }
        }
        "CreateLogStream" => {
            let req: CreateLogStreamRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterException",
                        &e.to_string(),
                    )
                }
            };
            match state.create_log_stream(req) {
                Ok(()) => (
                    StatusCode::OK,
                    [("content-type", "application/x-amz-json-1.1")],
                    "{}",
                )
                    .into_response(),
                Err(e) => map_logs_error(e),
            }
        }
        "DeleteLogStream" => {
            let req: DeleteLogStreamRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterException",
                        &e.to_string(),
                    )
                }
            };
            match state.delete_log_stream(req) {
                Ok(()) => (
                    StatusCode::OK,
                    [("content-type", "application/x-amz-json-1.1")],
                    "{}",
                )
                    .into_response(),
                Err(e) => map_logs_error(e),
            }
        }
        "DescribeLogStreams" => {
            let req: DescribeLogStreamsRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterException",
                        &e.to_string(),
                    )
                }
            };
            match state.describe_log_streams(req) {
                Ok((streams, next_token)) => json_response(
                    StatusCode::OK,
                    json!({
                        "logStreams": streams,
                        "nextToken": next_token
                    }),
                ),
                Err(e) => map_logs_error(e),
            }
        }
        "PutLogEvents" => {
            let req: PutLogEventsRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterException",
                        &e.to_string(),
                    )
                }
            };
            match state.put_log_events(req) {
                Ok(next_seq) => json_response(
                    StatusCode::OK,
                    json!({
                        "nextSequenceToken": next_seq
                    }),
                ),
                Err(e) => map_logs_error(e),
            }
        }
        "GetLogEvents" => {
            let req: GetLogEventsRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterException",
                        &e.to_string(),
                    )
                }
            };
            match state.get_log_events(req) {
                Ok(events) => json_response(
                    StatusCode::OK,
                    json!({
                        "events": events,
                        "nextForwardToken": "f/1",
                        "nextBackwardToken": "b/1"
                    }),
                ),
                Err(e) => map_logs_error(e),
            }
        }
        "FilterLogEvents" => {
            let req: FilterLogEventsRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterException",
                        &e.to_string(),
                    )
                }
            };
            match state.filter_log_events(req) {
                Ok(events) => json_response(
                    StatusCode::OK,
                    json!({
                        "events": events,
                        "searchedLogStreams": []
                    }),
                ),
                Err(e) => map_logs_error(e),
            }
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("Unknown CloudWatch Logs operation: {}", operation),
        ),
    }
}

fn json_response(status: StatusCode, val: serde_json::Value) -> Response {
    (
        status,
        [("content-type", "application/x-amz-json-1.1")],
        serde_json::to_string(&val).unwrap_or_default(),
    )
        .into_response()
}

fn error_response(status: StatusCode, error_type: &str, message: &str) -> Response {
    let body = json!({
        "__type": error_type,
        "message": message
    });
    (
        status,
        [("content-type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn map_logs_error(err: LogsError) -> Response {
    match err {
        LogsError::NotFound(msg) => {
            error_response(StatusCode::BAD_REQUEST, "ResourceNotFoundException", &msg)
        }
        LogsError::AlreadyExists(msg) => error_response(
            StatusCode::BAD_REQUEST,
            "ResourceAlreadyExistsException",
            &msg,
        ),
        LogsError::InvalidParameter(msg) => {
            error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &msg)
        }
        LogsError::InvalidSequenceToken(msg) => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidSequenceTokenException",
            &msg,
        ),
    }
}
