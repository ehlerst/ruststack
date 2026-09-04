use axum::extract::State;
use axum::http::HeaderMap;
use bytes::Bytes;
use ruststack_stepfunctions::{handle_stepfunctions_request, StepFunctionsState};
use serde_json::json;

#[tokio::test]
async fn test_stepfunctions_lifecycle_and_asl_execution() {
    let state = StepFunctionsState::new("000000000000", "us-east-1");

    let asl_definition = json!({
        "Comment": "Test State Machine",
        "StartAt": "InitialState",
        "States": {
            "InitialState": {
                "Type": "Pass",
                "Result": {"status": "in_progress", "value": 42},
                "Next": "CheckChoice"
            },
            "CheckChoice": {
                "Type": "Choice",
                "Choices": [
                    {
                        "Variable": "$.status",
                        "StringEquals": "in_progress",
                        "Next": "FinalSuccess"
                    }
                ],
                "Default": "FailState"
            },
            "FinalSuccess": {
                "Type": "Succeed"
            },
            "FailState": {
                "Type": "Fail",
                "Error": "InvalidStatus",
                "Cause": "Status was not in_progress"
            }
        }
    });

    // 1. CreateStateMachine
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-amz-target",
        "AWSStepFunctions.CreateStateMachine".parse().unwrap(),
    );
    let body = Bytes::from(
        json!({
            "Name": "order-pipeline",
            "Definition": asl_definition.to_string(),
            "RoleArn": "arn:aws:iam::000000000000:role/StepFunctionsRole"
        })
        .to_string(),
    );

    let resp = handle_stepfunctions_request(State(state.clone()), headers, body).await;
    assert_eq!(resp.status(), http::StatusCode::OK);

    // 2. DescribeStateMachine
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-amz-target",
        "AWSStepFunctions.DescribeStateMachine".parse().unwrap(),
    );
    let body = Bytes::from(
        json!({
            "StateMachineArn": "arn:aws:states:us-east-1:000000000000:stateMachine:order-pipeline"
        })
        .to_string(),
    );
    let resp = handle_stepfunctions_request(State(state.clone()), headers, body).await;
    assert_eq!(resp.status(), http::StatusCode::OK);

    // 3. StartExecution
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-amz-target",
        "AWSStepFunctions.StartExecution".parse().unwrap(),
    );
    let body = Bytes::from(
        json!({
            "StateMachineArn": "arn:aws:states:us-east-1:000000000000:stateMachine:order-pipeline",
            "Name": "exec-001",
            "Input": "{\"init\": true}"
        })
        .to_string(),
    );
    let resp = handle_stepfunctions_request(State(state.clone()), headers, body).await;
    assert_eq!(resp.status(), http::StatusCode::OK);

    // 4. DescribeExecution
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-amz-target",
        "AWSStepFunctions.DescribeExecution".parse().unwrap(),
    );
    let body = Bytes::from(
        json!({
            "ExecutionArn": "arn:aws:states:us-east-1:000000000000:execution:order-pipeline:exec-001"
        })
        .to_string(),
    );
    let resp = handle_stepfunctions_request(State(state.clone()), headers, body).await;
    assert_eq!(resp.status(), http::StatusCode::OK);

    // 5. GetExecutionHistory
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-amz-target",
        "AWSStepFunctions.GetExecutionHistory".parse().unwrap(),
    );
    let body = Bytes::from(
        json!({
            "ExecutionArn": "arn:aws:states:us-east-1:000000000000:execution:order-pipeline:exec-001"
        })
        .to_string(),
    );
    let resp = handle_stepfunctions_request(State(state.clone()), headers, body).await;
    assert_eq!(resp.status(), http::StatusCode::OK);
}
