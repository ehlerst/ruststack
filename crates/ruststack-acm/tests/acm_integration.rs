use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_acm::{handle_acm_request, AcmState};
use serde_json::json;

#[tokio::test]
async fn test_acm_certificate_lifecycle() {
    let state = AcmState::new("000000000000".to_string(), "us-east-1".to_string());
    let mut headers = HeaderMap::new();

    // 1. Request Certificate
    headers.insert("x-amz-target", "CertificateManager.RequestCertificate".parse().unwrap());
    let body = Bytes::from(
        json!({
            "DomainName": "api.example.com",
            "SubjectAlternativeNames": ["*.api.example.com"],
            "ValidationMethod": "DNS"
        })
        .to_string(),
    );
    let resp = handle_acm_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arn = json["CertificateArn"].as_str().unwrap();
    assert!(arn.contains("arn:aws:acm:us-east-1:000000000000:certificate/"));

    // 2. Describe Certificate
    headers.insert("x-amz-target", "CertificateManager.DescribeCertificate".parse().unwrap());
    let body = Bytes::from(json!({ "CertificateArn": arn }).to_string());
    let resp = handle_acm_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["Certificate"]["Status"], "ISSUED");
    assert_eq!(json["Certificate"]["DomainName"], "api.example.com");

    // 3. List Certificates
    headers.insert("x-amz-target", "CertificateManager.ListCertificates".parse().unwrap());
    let resp = handle_acm_request(State(state.clone()), headers.clone(), Bytes::new()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["CertificateSummaryList"].as_array().unwrap().len(), 1);
}
