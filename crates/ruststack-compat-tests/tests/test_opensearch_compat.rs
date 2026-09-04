use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_opensearch_domain_compat() {
    let client = RustStackTestClient::new();

    // 1. Create Domain
    let req = json!({
        "DomainName": "analytics-domain",
        "EngineVersion": "OpenSearch_2.11"
    });

    let (status, body) = client
        .call_rest(
            http::Method::POST,
            "/2021-01-01/opensearch/domain",
            Some(req.to_string()),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["domainStatus"]["domainName"], "analytics-domain");

    // 2. Describe Domain
    let (status, body) = client
        .call_rest(
            http::Method::GET,
            "/2021-01-01/opensearch/domain/analytics-domain",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["domainStatus"]["domainName"], "analytics-domain");
}
