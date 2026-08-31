use crate::state::{LambdaError, LambdaState};
use crate::types::*;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_lambda_request(
    State(state): State<LambdaState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // 1. Check for x-amz-target header
    if let Some(target) = headers.get("x-amz-target").and_then(|v| v.to_str().ok()) {
        if !target.is_empty() {
            let operation = if let Some(pos) = target.rfind('.') {
                &target[pos + 1..]
            } else {
                target
            };
            return handle_target_operation(&state, operation, &headers, &body);
        }
    }

    // 2. REST routing by HTTP Method and URI Path
    let path = uri.path();
    let clean_path = path.strip_prefix("/2015-03-31").unwrap_or(path);
    let segments: Vec<&str> = clean_path
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    if segments.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            "Missing Lambda action or resource path",
        );
    }

    match (method.clone(), segments.as_slice()) {
        // POST /2015-03-31/functions -> CreateFunction
        (Method::POST, ["functions"]) => {
            let req: CreateFunctionRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterValueException",
                        &e.to_string(),
                    )
                }
            };
            match state.create_function(req) {
                Ok(config) => json_response(StatusCode::CREATED, json!(config)),
                Err(e) => map_lambda_error(e),
            }
        }

        // GET /2015-03-31/functions -> ListFunctions
        (Method::GET, ["functions"]) => {
            let query = uri.query().unwrap_or_default();
            let mut marker = None;
            let mut max_items = None;
            for (k, v) in form_urlencoded::parse(query.as_bytes()) {
                if k.eq_ignore_ascii_case("Marker") {
                    marker = Some(v.into_owned());
                } else if k.eq_ignore_ascii_case("MaxItems") {
                    max_items = v.parse::<usize>().ok();
                }
            }
            match state.list_functions(marker, max_items) {
                Ok(resp) => json_response(StatusCode::OK, json!(resp)),
                Err(e) => map_lambda_error(e),
            }
        }

        // POST /2015-03-31/functions/{FunctionName}/invocations -> Invoke
        (Method::POST, ["functions", raw_name, "invocations"]) => {
            let function_name = decode_segment(raw_name);
            let inv_type_str = headers
                .get("x-amz-invocation-type")
                .and_then(|v| v.to_str().ok());

            let inv_type = inv_type_str.and_then(|s| s.parse::<InvocationType>().ok());

            match state.invoke_function(&function_name, Some(body.to_vec()), inv_type) {
                Ok(res) => {
                    let status = StatusCode::from_u16(res.status_code).unwrap_or(StatusCode::OK);

                    let mut resp_builder = Response::builder()
                        .status(status)
                        .header("content-type", "application/json")
                        .header("x-amz-executed-version", res.executed_version);

                    if let Some(err) = res.function_error {
                        resp_builder = resp_builder.header("x-amz-function-error", err);
                    }
                    if let Some(log) = res.log_result {
                        resp_builder = resp_builder.header("x-amz-log-result", log);
                    }

                    resp_builder
                        .body(axum::body::Body::from(res.payload))
                        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
                }
                Err(e) => map_lambda_error(e),
            }
        }

        // GET /2015-03-31/functions/{FunctionName}/configuration -> GetFunctionConfiguration
        (Method::GET, ["functions", raw_name, "configuration"]) => {
            let function_name = decode_segment(raw_name);
            match state.get_function_configuration(&function_name) {
                Ok(config) => json_response(StatusCode::OK, json!(config)),
                Err(e) => map_lambda_error(e),
            }
        }

        // GET /2015-03-31/functions/{FunctionName} -> GetFunction
        (Method::GET, ["functions", raw_name]) => {
            let function_name = decode_segment(raw_name);
            match state.get_function(&function_name) {
                Ok(resp) => json_response(StatusCode::OK, json!(resp)),
                Err(e) => map_lambda_error(e),
            }
        }

        // DELETE /2015-03-31/functions/{FunctionName} -> DeleteFunction
        (Method::DELETE, ["functions", raw_name]) => {
            let function_name = decode_segment(raw_name);
            let req = DeleteFunctionRequest {
                function_name,
                qualifier: None,
            };
            match state.delete_function(req) {
                Ok(()) => StatusCode::NO_CONTENT.into_response(),
                Err(e) => map_lambda_error(e),
            }
        }

        // POST /2015-03-31/event-source-mappings -> CreateEventSourceMapping
        (Method::POST, ["event-source-mappings"]) => {
            let req: CreateEventSourceMappingRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterValueException",
                        &e.to_string(),
                    )
                }
            };
            match state.create_event_source_mapping(req) {
                Ok(mapping) => json_response(StatusCode::ACCEPTED, json!(mapping)),
                Err(e) => map_lambda_error(e),
            }
        }

        // GET /2015-03-31/event-source-mappings -> ListEventSourceMappings
        (Method::GET, ["event-source-mappings"]) => {
            let query = uri.query().unwrap_or_default();
            let mut event_source_arn = None;
            let mut function_name = None;
            for (k, v) in form_urlencoded::parse(query.as_bytes()) {
                if k.eq_ignore_ascii_case("EventSourceArn") {
                    event_source_arn = Some(v.into_owned());
                } else if k.eq_ignore_ascii_case("FunctionName") {
                    function_name = Some(v.into_owned());
                }
            }
            match state
                .list_event_source_mappings(event_source_arn.as_deref(), function_name.as_deref())
            {
                Ok(resp) => json_response(StatusCode::OK, json!(resp)),
                Err(e) => map_lambda_error(e),
            }
        }

        // GET /2015-03-31/event-source-mappings/{UUID} -> GetEventSourceMapping
        (Method::GET, ["event-source-mappings", uuid]) => {
            match state.get_event_source_mapping(uuid) {
                Ok(mapping) => json_response(StatusCode::OK, json!(mapping)),
                Err(e) => map_lambda_error(e),
            }
        }

        // DELETE /2015-03-31/event-source-mappings/{UUID} -> DeleteEventSourceMapping
        (Method::DELETE, ["event-source-mappings", uuid]) => {
            match state.delete_event_source_mapping(uuid) {
                Ok(mapping) => json_response(StatusCode::ACCEPTED, json!(mapping)),
                Err(e) => map_lambda_error(e),
            }
        }

        _ => error_response(
            StatusCode::NOT_FOUND,
            "ResourceNotFoundException",
            &format!("Unknown Lambda resource or method: {} {}", method, path),
        ),
    }
}

fn handle_target_operation(
    state: &LambdaState,
    operation: &str,
    headers: &HeaderMap,
    body: &Bytes,
) -> Response {
    match operation {
        "CreateFunction" | "CreateFunction20150331" => {
            let req: CreateFunctionRequest = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterValueException",
                        &e.to_string(),
                    )
                }
            };
            match state.create_function(req) {
                Ok(config) => json_response(StatusCode::CREATED, json!(config)),
                Err(e) => map_lambda_error(e),
            }
        }
        "GetFunction" | "GetFunction20150331" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "PascalCase")]
            struct GetFnReq {
                #[serde(alias = "functionName", alias = "function_name")]
                function_name: String,
            }
            let req: GetFnReq = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterValueException",
                        &e.to_string(),
                    )
                }
            };
            match state.get_function(&req.function_name) {
                Ok(resp) => json_response(StatusCode::OK, json!(resp)),
                Err(e) => map_lambda_error(e),
            }
        }
        "GetFunctionConfiguration" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "PascalCase")]
            struct GetFnReq {
                #[serde(alias = "functionName", alias = "function_name")]
                function_name: String,
            }
            let req: GetFnReq = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterValueException",
                        &e.to_string(),
                    )
                }
            };
            match state.get_function_configuration(&req.function_name) {
                Ok(config) => json_response(StatusCode::OK, json!(config)),
                Err(e) => map_lambda_error(e),
            }
        }
        "DeleteFunction" | "DeleteFunction20150331" => {
            let req: DeleteFunctionRequest = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterValueException",
                        &e.to_string(),
                    )
                }
            };
            match state.delete_function(req) {
                Ok(()) => StatusCode::NO_CONTENT.into_response(),
                Err(e) => map_lambda_error(e),
            }
        }
        "ListFunctions" | "ListFunctions20150331" => match state.list_functions(None, None) {
            Ok(resp) => json_response(StatusCode::OK, json!(resp)),
            Err(e) => map_lambda_error(e),
        },
        "Invoke" | "InvokeAsync" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "PascalCase")]
            struct InvokeReq {
                #[serde(alias = "functionName", alias = "function_name")]
                function_name: String,
                #[serde(default, alias = "invocationType", alias = "invocation_type")]
                invocation_type: Option<InvocationType>,
                #[serde(default, alias = "payload")]
                payload: Option<serde_json::Value>,
            }

            let (fn_name, inv_type, payload_bytes) =
                if let Ok(req) = serde_json::from_slice::<InvokeReq>(body) {
                    let p_bytes = req.payload.map(|v| match v {
                        serde_json::Value::String(s) => s.into_bytes(),
                        other => serde_json::to_vec(&other).unwrap_or_default(),
                    });
                    (req.function_name, req.invocation_type, p_bytes)
                } else {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterValueException",
                        "Failed to parse invoke request",
                    );
                };

            let header_inv_type = headers
                .get("x-amz-invocation-type")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<InvocationType>().ok());

            let effective_inv_type = header_inv_type.or(inv_type);

            match state.invoke_function(&fn_name, payload_bytes, effective_inv_type) {
                Ok(res) => {
                    let status = StatusCode::from_u16(res.status_code).unwrap_or(StatusCode::OK);
                    let mut resp_builder = Response::builder()
                        .status(status)
                        .header("content-type", "application/json")
                        .header("x-amz-executed-version", res.executed_version);

                    if let Some(err) = res.function_error {
                        resp_builder = resp_builder.header("x-amz-function-error", err);
                    }
                    if let Some(log) = res.log_result {
                        resp_builder = resp_builder.header("x-amz-log-result", log);
                    }

                    resp_builder
                        .body(axum::body::Body::from(res.payload))
                        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
                }
                Err(e) => map_lambda_error(e),
            }
        }
        "CreateEventSourceMapping" => {
            let req: CreateEventSourceMappingRequest = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterValueException",
                        &e.to_string(),
                    )
                }
            };
            match state.create_event_source_mapping(req) {
                Ok(mapping) => json_response(StatusCode::ACCEPTED, json!(mapping)),
                Err(e) => map_lambda_error(e),
            }
        }
        "GetEventSourceMapping" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "PascalCase")]
            struct GetMappingReq {
                #[serde(alias = "UUID", alias = "uuid")]
                uuid: String,
            }
            let req: GetMappingReq = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterValueException",
                        &e.to_string(),
                    )
                }
            };
            match state.get_event_source_mapping(&req.uuid) {
                Ok(mapping) => json_response(StatusCode::OK, json!(mapping)),
                Err(e) => map_lambda_error(e),
            }
        }
        "DeleteEventSourceMapping" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "PascalCase")]
            struct DelMappingReq {
                #[serde(alias = "UUID", alias = "uuid")]
                uuid: String,
            }
            let req: DelMappingReq = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(e) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "InvalidParameterValueException",
                        &e.to_string(),
                    )
                }
            };
            match state.delete_event_source_mapping(&req.uuid) {
                Ok(mapping) => json_response(StatusCode::ACCEPTED, json!(mapping)),
                Err(e) => map_lambda_error(e),
            }
        }
        "ListEventSourceMappings" => {
            #[derive(serde::Deserialize, Default)]
            #[serde(rename_all = "PascalCase")]
            struct ListMappingReq {
                #[serde(default, alias = "eventSourceArn", alias = "event_source_arn")]
                event_source_arn: Option<String>,
                #[serde(default, alias = "functionName", alias = "function_name")]
                function_name: Option<String>,
            }
            let req: ListMappingReq = serde_json::from_slice(body).unwrap_or_default();
            match state.list_event_source_mappings(
                req.event_source_arn.as_deref(),
                req.function_name.as_deref(),
            ) {
                Ok(resp) => json_response(StatusCode::OK, json!(resp)),
                Err(e) => map_lambda_error(e),
            }
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("Unknown Lambda operation: {}", operation),
        ),
    }
}

fn decode_segment(s: &str) -> String {
    s.replace("%3A", ":")
        .replace("%3a", ":")
        .replace("%2F", "/")
        .replace("%2f", "/")
}

fn json_response(status: StatusCode, val: serde_json::Value) -> Response {
    (
        status,
        [("content-type", "application/json")],
        serde_json::to_string(&val).unwrap_or_default(),
    )
        .into_response()
}

fn error_response(status: StatusCode, error_type: &str, message: &str) -> Response {
    let body = json!({
        "__type": error_type,
        "Type": "User",
        "Message": message,
        "message": message
    });
    (
        status,
        [("content-type", "application/json")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn map_lambda_error(err: LambdaError) -> Response {
    match err {
        LambdaError::NotFound(msg) => {
            error_response(StatusCode::NOT_FOUND, "ResourceNotFoundException", &msg)
        }
        LambdaError::Conflict(msg) => {
            error_response(StatusCode::CONFLICT, "ResourceConflictException", &msg)
        }
        LambdaError::InvalidParameter(msg) => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidParameterValueException",
            &msg,
        ),
        LambdaError::InvalidRequestContent(msg) => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidRequestContentException",
            &msg,
        ),
        LambdaError::Service(msg) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "ServiceException", &msg)
        }
    }
}
