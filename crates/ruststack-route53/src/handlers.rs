use crate::state::{Route53Error, Route53State};
use crate::types::*;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_route53_request(
    State(state): State<Route53State>,
    method: Method,
    uri: Uri,
    _headers: HeaderMap,
    body: Bytes,
) -> Response {
    let path = uri.path();
    let segments: Vec<&str> = path.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();

    // Standard Route 53 URLs:
    // /2013-04-01/hostedzone
    // /2013-04-01/hostedzone/{id}
    // /2013-04-01/hostedzone/{id}/rrset
    // /hostedzone ...

    let clean_segments: Vec<&str> = if segments.first() == Some(&"2013-04-01") {
        segments[1..].to_vec()
    } else {
        segments
    };

    match (method, clean_segments.as_slice()) {
        // POST /hostedzone
        (Method::POST, ["hostedzone"]) => {
            let req: CreateHostedZoneRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidInput", &e.to_string()),
            };
            match state.create_hosted_zone(req) {
                Ok(res) => json_response(StatusCode::CREATED, serde_json::to_value(res).unwrap()),
                Err(e) => map_route53_error(e),
            }
        }
        // GET /hostedzone
        (Method::GET, ["hostedzone"]) => match state.list_hosted_zones() {
            Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
            Err(e) => map_route53_error(e),
        },
        // GET /hostedzone/{id}
        (Method::GET, ["hostedzone", zone_id]) => match state.get_hosted_zone(zone_id) {
            Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
            Err(e) => map_route53_error(e),
        },
        // DELETE /hostedzone/{id}
        (Method::DELETE, ["hostedzone", zone_id]) => match state.delete_hosted_zone(zone_id) {
            Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
            Err(e) => map_route53_error(e),
        },
        // POST /hostedzone/{id}/rrset
        (Method::POST, ["hostedzone", zone_id, "rrset"]) => {
            let req: ChangeResourceRecordSetsRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(e) => return error_response(StatusCode::BAD_REQUEST, "InvalidInput", &e.to_string()),
            };
            match state.change_resource_record_sets(zone_id, req) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_route53_error(e),
            }
        }
        // GET /hostedzone/{id}/rrset
        (Method::GET, ["hostedzone", zone_id, "rrset"]) => {
            match state.list_resource_record_sets(zone_id) {
                Ok(res) => json_response(StatusCode::OK, serde_json::to_value(res).unwrap()),
                Err(e) => map_route53_error(e),
            }
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            "Unsupported Route 53 action",
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

fn map_route53_error(err: Route53Error) -> Response {
    match err {
        Route53Error::NoSuchHostedZone(msg) => {
            error_response(StatusCode::NOT_FOUND, "NoSuchHostedZone", &msg)
        }
        Route53Error::HostedZoneAlreadyExists(msg) => {
            error_response(StatusCode::CONFLICT, "HostedZoneAlreadyExists", &msg)
        }
        Route53Error::InvalidInput(msg) => {
            error_response(StatusCode::BAD_REQUEST, "InvalidInput", &msg)
        }
        Route53Error::NoSuchChange(msg) => {
            error_response(StatusCode::NOT_FOUND, "NoSuchChange", &msg)
        }
    }
}
