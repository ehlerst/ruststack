use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_opensearch::{handle_opensearch_request, OpenSearchState};

#[tokio::test]
async fn test_opensearch_domain_lifecycle() {
    let state = OpenSearchState::new("000000000000".to_string(), "us-east-1".to_string());
    let headers = HeaderMap::new();

    // 1. Create Domain
    let uri: Uri = "/2021-01-01/opensearch/domain".parse().unwrap();
    let req_body = Bytes::from(serde_json::json!({
        "DomainName": "logs-domain",
        "EngineVersion": "OpenSearch_2.11"
    }).to_string());

    let resp = handle_opensearch_request(State(state.clone()), Method::POST, uri, headers.clone(), req_body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["domainStatus"]["domainName"], "logs-domain");
    assert!(json["domainStatus"]["endpoint"].as_str().unwrap().contains("logs-domain"));

    // 2. Describe Domain
    let uri: Uri = "/2021-01-01/opensearch/domain/logs-domain".parse().unwrap();
    let resp = handle_opensearch_request(State(state.clone()), Method::GET, uri, headers.clone(), Bytes::new()).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. List Domain Names
    let uri: Uri = "/2021-01-01/opensearch/domain-names".parse().unwrap();
    let resp = handle_opensearch_request(State(state.clone()), Method::GET, uri, headers.clone(), Bytes::new()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["domainNames"][0]["domainName"], "logs-domain");

    // 4. Delete Domain
    let uri: Uri = "/2021-01-01/opensearch/domain/logs-domain".parse().unwrap();
    let resp = handle_opensearch_request(State(state.clone()), Method::DELETE, uri, headers.clone(), Bytes::new()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
