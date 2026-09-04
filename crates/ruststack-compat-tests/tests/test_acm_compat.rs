use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_acm_request_and_describe_compat() {
    let client = RustStackTestClient::new();

    // 1. Request Certificate
    let (status, body) = client
        .call_json(
            "CertificateManager.RequestCertificate",
            json!({
                "DomainName": "app.domain.com",
                "SubjectAlternativeNames": ["*.app.domain.com"],
                "ValidationMethod": "DNS"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let arn = body["CertificateArn"].as_str().unwrap();
    assert!(arn.contains("arn:aws:acm:"));

    // 2. Describe Certificate
    let (status, body) = client
        .call_json(
            "CertificateManager.DescribeCertificate",
            json!({
                "CertificateArn": arn
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["Certificate"]["DomainName"], "app.domain.com");
    assert_eq!(body["Certificate"]["Status"], "ISSUED");
    assert!(body["Certificate"]["DomainValidationOptions"].as_array().unwrap().len() >= 1);

    // 3. List Certificates
    let (status, body) = client
        .call_json(
            "CertificateManager.ListCertificates",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["CertificateSummaryList"].as_array().unwrap().len(), 1);
}
