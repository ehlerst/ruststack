use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_msk::{handle_msk_request, MskState};
use serde_json::json;

#[tokio::test]
async fn test_msk_cluster_lifecycle() {
    let state = MskState::new("000000000000".to_string(), "us-east-1".to_string());
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());

    // 1. Create Cluster
    let body = Bytes::from(
        json!({
            "ClusterName": "demo-kafka",
            "NumberOfBrokerNodes": 3,
            "KafkaVersion": "3.5.1"
        })
        .to_string(),
    );
    let uri: Uri = "/v1/clusters".parse().unwrap();
    let resp = handle_msk_request(
        State(state.clone()),
        Method::POST,
        uri,
        headers.clone(),
        body,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arn = val["ClusterArn"].as_str().unwrap();
    assert!(arn.contains("arn:aws:kafka:"));

    // 2. Describe Cluster
    let uri: Uri = format!("/v1/clusters/{}", arn).parse().unwrap();
    let resp = handle_msk_request(
        State(state.clone()),
        Method::GET,
        uri,
        headers.clone(),
        Bytes::new(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["ClusterInfo"]["ClusterName"], "demo-kafka");
    assert_eq!(val["ClusterInfo"]["State"], "ACTIVE");

    // 3. Get Bootstrap Brokers
    let uri: Uri = format!("/v1/clusters/{}/bootstrap-brokers", arn)
        .parse()
        .unwrap();
    let resp = handle_msk_request(
        State(state.clone()),
        Method::GET,
        uri,
        headers.clone(),
        Bytes::new(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(val["BootstrapBrokerString"]
        .as_str()
        .unwrap()
        .contains(":9092"));

    // 4. List Clusters
    let uri: Uri = "/v1/clusters".parse().unwrap();
    let resp = handle_msk_request(
        State(state.clone()),
        Method::GET,
        uri,
        headers.clone(),
        Bytes::new(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["ClusterInfoList"].as_array().unwrap().len(), 1);

    // 5. Delete Cluster
    let uri: Uri = format!("/v1/clusters/{}", arn).parse().unwrap();
    let resp = handle_msk_request(
        State(state.clone()),
        Method::DELETE,
        uri,
        headers.clone(),
        Bytes::new(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}
