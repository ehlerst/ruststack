use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_ecr_docker_repository_compat() {
    let client = RustStackTestClient::new();

    // 1. CreateRepository
    let (status, body) = client
        .call_json(
            "AmazonEC2ContainerRegistry_V20150921.CreateRepository",
            json!({
                "repositoryName": "service-image"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["repository"]["repositoryName"], "service-image");

    // 2. PutImage
    let (status, _) = client
        .call_json(
            "AmazonEC2ContainerRegistry_V20150921.PutImage",
            json!({
                "repositoryName": "service-image",
                "imageManifest": "{\"config\":{}}",
                "imageTag": "latest"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 3. ListImages
    let (status, body) = client
        .call_json(
            "AmazonEC2ContainerRegistry_V20150921.ListImages",
            json!({
                "repositoryName": "service-image"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let ids = body["imageIds"].as_array().unwrap();
    assert!(ids.iter().any(|i| i["imageTag"] == "latest"));
}
