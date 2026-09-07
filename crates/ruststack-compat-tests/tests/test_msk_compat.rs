use http::{Method, StatusCode};
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_msk_lifecycle_compat() {
    let client = RustStackTestClient::new();

    // 1. Create Cluster
    let (status, body) = client
        .call_rest_json(
            Method::POST,
            "/v1/clusters",
            Some(json!({
                "ClusterName": "analytics-kafka",
                "NumberOfBrokerNodes": 3,
                "KafkaVersion": "3.5.1"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let cluster_arn = body["ClusterArn"].as_str().unwrap();
    assert_eq!(body["ClusterName"], "analytics-kafka");
    assert_eq!(body["State"], "ACTIVE");

    // 2. Describe Cluster
    let describe_uri = format!("/v1/clusters/{}", cluster_arn);
    let (status, body) = client
        .call_rest_json(Method::GET, &describe_uri, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ClusterInfo"]["ClusterName"], "analytics-kafka");
    assert_eq!(body["ClusterInfo"]["NumberOfBrokerNodes"], 3);

    // 3. Get Bootstrap Brokers
    let brokers_uri = format!("/v1/clusters/{}/bootstrap-brokers", cluster_arn);
    let (status, body) = client.call_rest_json(Method::GET, &brokers_uri, None).await;
    assert_eq!(status, StatusCode::OK);
    let bootstrap_str = body["BootstrapBrokerString"].as_str().unwrap();
    assert!(bootstrap_str.contains("9092"));

    // 4. List Clusters
    let (status, body) = client
        .call_rest_json(Method::GET, "/v1/clusters", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let clusters = body["ClusterInfoList"].as_array().unwrap();
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0]["ClusterName"], "analytics-kafka");

    // 5. Delete Cluster
    let (status, body) = client
        .call_rest_json(Method::DELETE, &describe_uri, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["State"], "DELETING");

    // 6. Verify List is now empty
    let (status, body) = client
        .call_rest_json(Method::GET, "/v1/clusters", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ClusterInfoList"].as_array().unwrap().len(), 0);
}
