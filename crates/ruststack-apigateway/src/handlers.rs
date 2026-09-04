use crate::state::{ApiGatewayError, ApiGatewayState};
use crate::types::*;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde_json::json;
use std::collections::HashMap;

pub async fn handle_apigateway_request(
    State(state): State<ApiGatewayState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let path = uri.path();
    let segments: Vec<&str> = path.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();

    // 1. Control plane REST API: /restapis
    if segments.first() == Some(&"restapis") {
        return handle_restapis_control_plane(&state, &method, &segments, &body);
    }

    // 2. Invocation routing: /{api_id}/{stage}/{path...}
    if segments.len() >= 2 && !segments[0].starts_with('_') {
        let api_id = segments[0];
        let stage = segments[1];
        let req_path = if segments.len() > 2 {
            format!("/{}", segments[2..].join("/"))
        } else {
            "/".to_string()
        };

        let mut header_map = HashMap::new();
        for (k, v) in headers.iter() {
            if let Ok(s) = v.to_str() {
                header_map.insert(k.as_str().to_string(), s.to_string());
            }
        }

        match state.invoke_api(
            api_id,
            stage,
            method.as_str(),
            &req_path,
            &header_map,
            &body,
        ) {
            Ok((status_code, resp_headers, resp_body)) => {
                let status = StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK);
                let mut res = Response::new(axum::body::Body::from(resp_body));
                *res.status_mut() = status;
                for (k, v) in resp_headers {
                    if let (Ok(hname), Ok(hval)) = (
                        axum::http::HeaderName::from_bytes(k.as_bytes()),
                        axum::http::HeaderValue::from_str(&v),
                    ) {
                        res.headers_mut().insert(hname, hval);
                    }
                }
                return res;
            }
            Err(e) => return map_apigateway_error(e),
        }
    }

    error_response(StatusCode::NOT_FOUND, "NotFoundException", "Endpoint not found")
}

fn handle_restapis_control_plane(
    state: &ApiGatewayState,
    method: &Method,
    segments: &[&str],
    body: &[u8],
) -> Response {
    match (method, segments) {
        // GET /restapis
        (&Method::GET, ["restapis"]) => match state.get_rest_apis() {
            Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
            Err(e) => map_apigateway_error(e),
        },
        // POST /restapis
        (&Method::POST, ["restapis"]) => {
            let req: CreateRestApiRequest = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "BadRequestException", &e.to_string()),
            };
            match state.create_rest_api(req) {
                Ok(res) => json_response(StatusCode::CREATED, serde_json::to_value(res).unwrap()),
                Err(e) => map_apigateway_error(e),
            }
        }
        // GET /restapis/{api_id}
        (&Method::GET, ["restapis", api_id]) => match state.get_rest_api(api_id) {
            Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
            Err(e) => map_apigateway_error(e),
        },
        // DELETE /restapis/{api_id}
        (&Method::DELETE, ["restapis", api_id]) => match state.delete_rest_api(api_id) {
            Ok(()) => json_response(StatusCode::ACCEPTED, json!({})),
            Err(e) => map_apigateway_error(e),
        },
        // GET /restapis/{api_id}/resources
        (&Method::GET, ["restapis", api_id, "resources"]) => match state.get_resources(api_id) {
            Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
            Err(e) => map_apigateway_error(e),
        },
        // POST /restapis/{api_id}/resources/{parent_id}
        (&Method::POST, ["restapis", api_id, "resources", parent_id]) => {
            let req: CreateResourceRequest = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "BadRequestException", &e.to_string()),
            };
            match state.create_resource(api_id, parent_id, &req.path_part) {
                Ok(res) => json_response(StatusCode::CREATED, serde_json::to_value(res).unwrap()),
                Err(e) => map_apigateway_error(e),
            }
        }
        // GET /restapis/{api_id}/resources/{resource_id}
        (&Method::GET, ["restapis", api_id, "resources", resource_id]) => {
            match state.get_resource(api_id, resource_id) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_apigateway_error(e),
            }
        }
        // PUT /restapis/{api_id}/resources/{resource_id}/methods/{http_method}
        (&Method::PUT, ["restapis", api_id, "resources", resource_id, "methods", http_method]) => {
            let req: PutMethodRequest = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(_) => PutMethodRequest {
                    authorization_type: Some("NONE".to_string()),
                    request_parameters: None,
                },
            };
            match state.put_method(api_id, resource_id, http_method, req) {
                Ok(res) => json_response(StatusCode::CREATED, serde_json::to_value(res).unwrap()),
                Err(e) => map_apigateway_error(e),
            }
        }
        // PUT /restapis/{api_id}/resources/{resource_id}/methods/{http_method}/integration
        (
            &Method::PUT,
            [
                "restapis",
                api_id,
                "resources",
                resource_id,
                "methods",
                http_method,
                "integration",
            ],
        ) => {
            let req: PutIntegrationRequest = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "BadRequestException", &e.to_string()),
            };
            match state.put_integration(api_id, resource_id, http_method, req) {
                Ok(res) => json_response(StatusCode::CREATED, serde_json::to_value(res).unwrap()),
                Err(e) => map_apigateway_error(e),
            }
        }
        // POST /restapis/{api_id}/deployments
        (&Method::POST, ["restapis", api_id, "deployments"]) => {
            let req: CreateDeploymentRequest = match serde_json::from_slice(body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "BadRequestException", &e.to_string()),
            };
            match state.create_deployment(api_id, req) {
                Ok(res) => json_response(StatusCode::CREATED, serde_json::to_value(res).unwrap()),
                Err(e) => map_apigateway_error(e),
            }
        }
        // GET /restapis/{api_id}/stages
        (&Method::GET, ["restapis", api_id, "stages"]) => match state.get_stages(api_id) {
            Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
            Err(e) => map_apigateway_error(e),
        },
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            "Unsupported API Gateway action",
        ),
    }
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
        "message": message,
        "__type": error_type
    });
    (
        status,
        [("content-type", "application/json")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn map_apigateway_error(err: ApiGatewayError) -> Response {
    match err {
        ApiGatewayError::NotFound(msg) => {
            error_response(StatusCode::NOT_FOUND, "NotFoundException", &msg)
        }
        ApiGatewayError::BadRequest(msg) => {
            error_response(StatusCode::BAD_REQUEST, "BadRequestException", &msg)
        }
        ApiGatewayError::Conflict(msg) => {
            error_response(StatusCode::CONFLICT, "ConflictException", &msg)
        }
    }
}
