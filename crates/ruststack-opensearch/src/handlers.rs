use crate::state::OpenSearchState;
use crate::types::*;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Method, Response, StatusCode, Uri};
use bytes::Bytes;
use serde_json::json;

pub async fn handle_opensearch_request(
    State(state): State<OpenSearchState>,
    method: Method,
    uri: Uri,
    _headers: HeaderMap,
    body_bytes: Bytes,
) -> Response<Body> {
    let path = uri.path();

    // 1. Create Domain POST /2021-01-01/opensearch/domain
    if method == Method::POST && (path == "/2021-01-01/opensearch/domain" || path == "/opensearch/domain" || path == "/2015-01-01/es/domain") {
        let req: CreateDomainRequest = match serde_json::from_slice(&body_bytes) {
            Ok(r) => r,
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "message": format!("Invalid JSON: {}", e) }).to_string()))
                    .unwrap();
            }
        };

        match state.create_domain(req) {
            Ok(domain_status) => {
                let resp = CreateDomainResponse { domain_status };
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&resp).unwrap()))
                    .unwrap();
            }
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "message": e.to_string() }).to_string()))
                    .unwrap();
            }
        }
    }

    // 2. List Domain Names GET /2021-01-01/opensearch/domain-names
    if method == Method::GET && (path == "/2021-01-01/opensearch/domain-names" || path == "/opensearch/domain-names" || path == "/2015-01-01/es/domain-names") {
        let domain_names = state.list_domain_names();
        let resp = ListDomainNamesResponse { domain_names };
        return Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&resp).unwrap()))
            .unwrap();
    }

    // 3. Describe Domain GET /2021-01-01/opensearch/domain/{DomainName}
    if method == Method::GET && (path.contains("/opensearch/domain/") || path.contains("/es/domain/")) {
        let domain_name = path.rsplit('/').next().unwrap_or("");
        match state.describe_domain(domain_name) {
            Ok(domain_status) => {
                let resp = DescribeDomainResponse { domain_status };
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&resp).unwrap()))
                    .unwrap();
            }
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "message": e.to_string() }).to_string()))
                    .unwrap();
            }
        }
    }

    // 4. Delete Domain DELETE /2021-01-01/opensearch/domain/{DomainName}
    if method == Method::DELETE && (path.contains("/opensearch/domain/") || path.contains("/es/domain/")) {
        let domain_name = path.rsplit('/').next().unwrap_or("");
        match state.delete_domain(domain_name) {
            Ok(domain_status) => {
                let resp = DeleteDomainResponse { domain_status };
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&resp).unwrap()))
                    .unwrap();
            }
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "message": e.to_string() }).to_string()))
                    .unwrap();
            }
        }
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("content-type", "application/json")
        .body(Body::from(json!({ "message": "Unknown OpenSearch operation or path" }).to_string()))
        .unwrap()
}
