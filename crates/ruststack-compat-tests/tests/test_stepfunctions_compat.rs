use http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_stepfunctions_state_machine_execution_compat() {
    let client = RustStackTestClient::new();

    let asl = json!({
        "Comment": "Compat Workflow",
        "StartAt": "Compute",
        "States": {
            "Compute": {
                "Type": "Pass",
                "Result": {"result": "success", "count": 10},
                "Next": "Evaluate"
            },
            "Evaluate": {
                "Type": "Choice",
                "Choices": [
                    {
                        "Variable": "$.result",
                        "StringEquals": "success",
                        "Next": "Done"
                    }
                ],
                "Default": "FailState"
            },
            "Done": {
                "Type": "Succeed"
            },
            "FailState": {
                "Type": "Fail"
            }
        }
    });

    // 1. CreateStateMachine
    let (status, body) = client
        .call_json(
            "AWSStepFunctions.CreateStateMachine",
            json!({
                "Name": "order-orchestrator",
                "Definition": asl.to_string(),
                "RoleArn": "arn:aws:iam::000000000000:role/StepFunctionsExecutionRole"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let sm_arn = body["StateMachineArn"].as_str().unwrap();

    // 2. StartExecution
    let (status, body) = client
        .call_json(
            "AWSStepFunctions.StartExecution",
            json!({
                "StateMachineArn": sm_arn,
                "Name": "test-run-1"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let exec_arn = body["ExecutionArn"].as_str().unwrap();

    // 3. DescribeExecution
    let (status, body) = client
        .call_json(
            "AWSStepFunctions.DescribeExecution",
            json!({
                "ExecutionArn": exec_arn
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["Status"].as_str().unwrap(), "SUCCEEDED");
}
