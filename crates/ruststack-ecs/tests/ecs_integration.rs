use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use bytes::Bytes;
use http_body_util::BodyExt;
use ruststack_ecs::{handle_ecs_request, EcsState};
use serde_json::json;

#[tokio::test]
async fn test_ecs_full_lifecycle() {
    let state = EcsState::new("000000000000".to_string(), "us-east-1".to_string());

    // 1. Create Cluster
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-amz-target",
        HeaderValue::from_static("AmazonEC2ContainerServiceV20141113.CreateCluster"),
    );
    let body = Bytes::from(json!({ "clusterName": "prod-cluster" }).to_string());
    let resp = handle_ecs_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Register Task Definition
    headers.insert(
        "x-amz-target",
        HeaderValue::from_static("AmazonEC2ContainerServiceV20141113.RegisterTaskDefinition"),
    );
    let body = Bytes::from(
        json!({
            "family": "web-app",
            "containerDefinitions": [
                {
                    "name": "web",
                    "image": "nginx:alpine",
                    "cpu": 256,
                    "memory": 512,
                    "essential": true,
                    "portMappings": [
                        { "containerPort": 80, "hostPort": 80, "protocol": "tcp" }
                    ]
                }
            ]
        })
        .to_string(),
    );
    let resp = handle_ecs_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Run Task
    headers.insert(
        "x-amz-target",
        HeaderValue::from_static("AmazonEC2ContainerServiceV20141113.RunTask"),
    );
    let body = Bytes::from(
        json!({
            "cluster": "prod-cluster",
            "taskDefinition": "web-app:1",
            "count": 2
        })
        .to_string(),
    );
    let resp = handle_ecs_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let tasks = val["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 2);
    let task_arn = tasks[0]["taskArn"].as_str().unwrap();

    // 4. Describe Tasks
    headers.insert(
        "x-amz-target",
        HeaderValue::from_static("AmazonEC2ContainerServiceV20141113.DescribeTasks"),
    );
    let body = Bytes::from(
        json!({
            "cluster": "prod-cluster",
            "tasks": [task_arn]
        })
        .to_string(),
    );
    let resp = handle_ecs_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. Create Service
    headers.insert(
        "x-amz-target",
        HeaderValue::from_static("AmazonEC2ContainerServiceV20141113.CreateService"),
    );
    let body = Bytes::from(
        json!({
            "cluster": "prod-cluster",
            "serviceName": "web-service",
            "taskDefinition": "web-app:1",
            "desiredCount": 3
        })
        .to_string(),
    );
    let resp = handle_ecs_request(State(state.clone()), headers.clone(), body).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
