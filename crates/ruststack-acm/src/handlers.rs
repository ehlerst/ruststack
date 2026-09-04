use crate::state::AcmState;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode};
use bytes::Bytes;
use serde_json::{json, Value};

pub async fn handle_acm_request(
    State(state): State<AcmState>,
    headers: HeaderMap,
    body_bytes: Bytes,
) -> Response<Body> {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let action = target
        .split('.')
        .nth(1)
        .or_else(|| target.split('#').nth(1))
        .unwrap_or(target);

    let val: Value = serde_json::from_slice(&body_bytes).unwrap_or(Value::Null);

    match action {
        "RequestCertificate" => {
            let domain_name = val["DomainName"].as_str().unwrap_or_default().to_string();
            let sans = val["SubjectAlternativeNames"].as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });
            let validation_method = val["ValidationMethod"].as_str().map(String::from);
            let key_algorithm = val["KeyAlgorithm"].as_str().map(String::from);

            match state.request_certificate(domain_name, sans, validation_method, key_algorithm) {
                Ok(arn) => {
                    let resp = json!({
                        "CertificateArn": arn
                    });
                    json_response(StatusCode::OK, resp)
                }
                Err(e) => error_response(StatusCode::BAD_REQUEST, "InvalidParameterException", &e.to_string()),
            }
        }
        "DescribeCertificate" => {
            let cert_arn = val["CertificateArn"].as_str().unwrap_or_default();
            match state.describe_certificate(cert_arn) {
                Ok(cert) => {
                    let resp = json!({
                        "Certificate": cert
                    });
                    json_response(StatusCode::OK, resp)
                }
                Err(e) => error_response(StatusCode::NOT_FOUND, "ResourceNotFoundException", &e.to_string()),
            }
        }
        "ListCertificates" => {
            let certs = state.list_certificates();
            let resp = json!({
                "CertificateSummaryList": certs
            });
            json_response(StatusCode::OK, resp)
        }
        "DeleteCertificate" => {
            let cert_arn = val["CertificateArn"].as_str().unwrap_or_default();
            match state.delete_certificate(cert_arn) {
                Ok(_) => json_response(StatusCode::OK, json!({})),
                Err(e) => error_response(StatusCode::NOT_FOUND, "ResourceNotFoundException", &e.to_string()),
            }
        }
        "GetCertificate" => {
            let cert_arn = val["CertificateArn"].as_str().unwrap_or_default();
            match state.get_certificate(cert_arn) {
                Ok((cert, chain)) => {
                    let resp = json!({
                        "Certificate": cert,
                        "CertificateChain": chain
                    });
                    json_response(StatusCode::OK, resp)
                }
                Err(e) => error_response(StatusCode::NOT_FOUND, "ResourceNotFoundException", &e.to_string()),
            }
        }
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidAction",
            &format!("Unknown action {}", action),
        ),
    }
}

fn json_response(status: StatusCode, val: Value) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(val.to_string()))
        .unwrap()
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response<Body> {
    let err = json!({
        "__type": code,
        "message": message
    });
    Response::builder()
        .status(status)
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(err.to_string()))
        .unwrap()
}
