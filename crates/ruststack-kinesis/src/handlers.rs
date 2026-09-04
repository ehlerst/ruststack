use crate::state::{KinesisError, KinesisState};
use crate::types::*;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_kinesis_request(
    State(state): State<KinesisState>,
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
        "CreateStream" => {
            let req: CreateStreamRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.create_stream(req) {
                Ok(()) => (
                    StatusCode::OK,
                    [("content-type", "application/x-amz-json-1.1")],
                    "{}",
                )
                    .into_response(),
                Err(e) => map_kinesis_error(e),
            }
        }
        "DeleteStream" => {
            let req: DeleteStreamRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.delete_stream(req) {
                Ok(()) => (
                    StatusCode::OK,
                    [("content-type", "application/x-amz-json-1.1")],
                    "{}",
                )
                    .into_response(),
                Err(e) => map_kinesis_error(e),
            }
        }
        "DescribeStream" => {
            let req: DescribeStreamRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.describe_stream(req) {
                Ok(desc) => json_response(
                    StatusCode::OK,
                    json!({
                        "StreamDescription": desc
                    }),
                ),
                Err(e) => map_kinesis_error(e),
            }
        }
        "DescribeStreamSummary" => {
            let req: DescribeStreamSummaryRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.describe_stream_summary(req) {
                Ok(summary) => json_response(
                    StatusCode::OK,
                    json!({
                        "StreamDescriptionSummary": summary
                    }),
                ),
                Err(e) => map_kinesis_error(e),
            }
        }
        "ListStreams" => {
            let req: ListStreamsRequest = serde_json::from_slice(&body).unwrap_or_default();
            match state.list_streams(req) {
                Ok(res) => json_response(
                    StatusCode::OK,
                    serde_json::to_value(&res).unwrap_or_default(),
                ),
                Err(e) => map_kinesis_error(e),
            }
        }
        "PutRecord" => {
            let req: PutRecordRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.put_record(req) {
                Ok(res) => json_response(
                    StatusCode::OK,
                    serde_json::to_value(&res).unwrap_or_default(),
                ),
                Err(e) => map_kinesis_error(e),
            }
        }
        "PutRecords" => {
            let req: PutRecordsRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.put_records(req) {
                Ok(res) => json_response(
                    StatusCode::OK,
                    serde_json::to_value(&res).unwrap_or_default(),
                ),
                Err(e) => map_kinesis_error(e),
            }
        }
        "GetShardIterator" => {
            let req: GetShardIteratorRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.get_shard_iterator(req) {
                Ok(res) => json_response(
                    StatusCode::OK,
                    serde_json::to_value(&res).unwrap_or_default(),
                ),
                Err(e) => map_kinesis_error(e),
            }
        }
        "GetRecords" => {
            let req: GetRecordsRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.get_records(req) {
                Ok(res) => json_response(
                    StatusCode::OK,
                    serde_json::to_value(&res).unwrap_or_default(),
                ),
                Err(e) => map_kinesis_error(e),
            }
        }
        "AddTagsToStream" => {
            let req: AddTagsToStreamRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.add_tags_to_stream(req) {
                Ok(()) => (
                    StatusCode::OK,
                    [("content-type", "application/x-amz-json-1.1")],
                    "{}",
                )
                    .into_response(),
                Err(e) => map_kinesis_error(e),
            }
        }
        "RemoveTagsFromStream" => {
            let req: RemoveTagsFromStreamRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.remove_tags_from_stream(req) {
                Ok(()) => (
                    StatusCode::OK,
                    [("content-type", "application/x-amz-json-1.1")],
                    "{}",
                )
                    .into_response(),
                Err(e) => map_kinesis_error(e),
            }
        }
        "ListTagsForStream" => {
            let req: ListTagsForStreamRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.list_tags_for_stream(req) {
                Ok(res) => json_response(
                    StatusCode::OK,
                    serde_json::to_value(&res).unwrap_or_default(),
                ),
                Err(e) => map_kinesis_error(e),
            }
        }
        "SplitShard" => {
            let req: SplitShardRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.split_shard(req) {
                Ok(()) => (
                    StatusCode::OK,
                    [("content-type", "application/x-amz-json-1.1")],
                    "{}",
                )
                    .into_response(),
                Err(e) => map_kinesis_error(e),
            }
        }
        "MergeShards" => {
            let req: MergeShardsRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.merge_shards(req) {
                Ok(()) => (
                    StatusCode::OK,
                    [("content-type", "application/x-amz-json-1.1")],
                    "{}",
                )
                    .into_response(),
                Err(e) => map_kinesis_error(e),
            }
        }
        "UpdateShardCount" => {
            let req: UpdateShardCountRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidArgumentException",
                        &e.to_string(),
                    )
                }
            };
            match state.update_shard_count(req) {
                Ok(res) => json_response(
                    StatusCode::OK,
                    serde_json::to_value(&res).unwrap_or_default(),
                ),
                Err(e) => map_kinesis_error(e),
            }
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("Unknown Kinesis operation: {}", operation),
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

fn map_kinesis_error(err: KinesisError) -> Response {
    match err {
        KinesisError::ResourceNotFound(msg) => {
            error_response(StatusCode::BAD_REQUEST, "ResourceNotFoundException", &msg)
        }
        KinesisError::ResourceInUse(msg) => {
            error_response(StatusCode::BAD_REQUEST, "ResourceInUseException", &msg)
        }
        KinesisError::InvalidArgument(msg) => {
            error_response(StatusCode::BAD_REQUEST, "InvalidArgumentException", &msg)
        }
        KinesisError::ExpiredIterator(msg) => {
            error_response(StatusCode::BAD_REQUEST, "ExpiredIteratorException", &msg)
        }
        KinesisError::LimitExceeded(msg) => {
            error_response(StatusCode::BAD_REQUEST, "LimitExceededException", &msg)
        }
        KinesisError::Validation(msg) => {
            error_response(StatusCode::BAD_REQUEST, "ValidationException", &msg)
        }
    }
}
