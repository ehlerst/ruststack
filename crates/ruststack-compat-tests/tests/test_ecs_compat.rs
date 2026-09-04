use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_ecs_fargate_cluster_task_service_compat() {
    let client = RustStackTestClient::new();

    // 1. CreateCluster
    let (status, body) = client
        .call_json(
            "AmazonEC2ContainerServiceV20141113.CreateCluster",
            json!({
                "clusterName": "compat-ecs-cluster"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["cluster"]["clusterName"], "compat-ecs-cluster");

    // 2. RegisterTaskDefinition
    let (status, body) = client
        .call_json(
            "AmazonEC2ContainerServiceV20141113.RegisterTaskDefinition",
            json!({
                "family": "api-service",
                "containerDefinitions": [
                    {
                        "name": "api",
                        "image": "my-api:v1.0.0",
                        "cpu": 256,
                        "memory": 512,
                        "essential": true
                    }
                ]
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let td_arn = body["taskDefinition"]["taskDefinitionArn"].as_str().unwrap();

    // 3. CreateService
    let (status, body) = client
        .call_json(
            "AmazonEC2ContainerServiceV20141113.CreateService",
            json!({
                "cluster": "compat-ecs-cluster",
                "serviceName": "api-srv",
                "taskDefinition": td_arn,
                "desiredCount": 2
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["service"]["serviceName"], "api-srv");
    assert_eq!(body["service"]["desiredCount"], 2);
}
