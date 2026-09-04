use ruststack_lambda::types::*;
use ruststack_lambda::LambdaState;

#[test]
fn test_lambda_function_and_invocation_lifecycle() {
    let state = LambdaState::new("000000000000".to_string(), "us-east-1".to_string());

    // 1. Create Function
    let create_req = CreateFunctionRequest {
        function_name: "my-test-func".to_string(),
        runtime: Some("nodejs18.x".to_string()),
        role: "arn:aws:iam::000000000000:role/lambda-role".to_string(),
        handler: Some("index.handler".to_string()),
        code: Some(FunctionCode::default()),
        description: Some("Test lambda".to_string()),
        timeout: Some(30),
        memory_size: Some(128),
        publish: Some(true),
        environment: None,
        tags: None,
        architectures: None,
        ephemeral_storage: None,
        package_type: None,
        tracing_config: None,
    };
    let config = state.create_function(create_req).unwrap();
    assert_eq!(config.function_name, "my-test-func");

    // 2. Get Function
    let get_resp = state.get_function("my-test-func").unwrap();
    assert_eq!(
        get_resp.configuration.unwrap().function_name,
        "my-test-func"
    );

    // 3. List Functions
    let list_resp = state.list_functions(None, None).unwrap();
    assert_eq!(list_resp.functions.len(), 1);

    // 4. Invoke Function (RequestResponse)
    let payload = serde_json::json!({ "name": "Alice" }).to_string();
    let res = state
        .invoke_function(
            "my-test-func",
            Some(payload.into_bytes()),
            Some(InvocationType::RequestResponse),
        )
        .unwrap();
    assert_eq!(res.status_code, 200);
    assert!(res.function_error.is_none());
    assert!(!res.payload.is_empty());

    // 5. Delete Function
    state
        .delete_function(DeleteFunctionRequest {
            function_name: "my-test-func".to_string(),
            qualifier: None,
        })
        .unwrap();
    let list_after = state.list_functions(None, None).unwrap();
    assert_eq!(list_after.functions.len(), 0);
}

#[tokio::test]
async fn test_lambda_active_sqs_event_source_poller() {
    let state = std::sync::Arc::new(LambdaState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let sqs = std::sync::Arc::new(ruststack_sqs::SqsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    state.set_sqs_engine(sqs.clone());

    // 1. Create Lambda Function
    let create_req = CreateFunctionRequest {
        function_name: "queue-worker".to_string(),
        runtime: Some("nodejs18.x".to_string()),
        role: "arn:aws:iam::000000000000:role/lambda-role".to_string(),
        handler: Some("index.handler".to_string()),
        code: Some(FunctionCode::default()),
        description: Some("Worker".to_string()),
        timeout: Some(30),
        memory_size: Some(128),
        publish: Some(true),
        environment: None,
        tags: None,
        architectures: None,
        ephemeral_storage: None,
        package_type: None,
        tracing_config: None,
    };
    state.create_function(create_req).unwrap();

    // 2. Create SQS Queue
    let queue_url = sqs.create_queue("work-items", None).unwrap();
    let queue_arn = "arn:aws:sqs:us-east-1:000000000000:work-items";

    // 3. Create EventSourceMapping
    let esm_req = CreateEventSourceMappingRequest {
        event_source_arn: queue_arn.to_string(),
        function_name: "queue-worker".to_string(),
        enabled: Some(true),
        batch_size: Some(5),
        starting_position: Some("LATEST".to_string()),
        starting_position_timestamp: None,
        maximum_batching_window_in_seconds: None,
    };
    state.create_event_source_mapping(esm_req).unwrap();

    // 4. Send 3 messages to SQS
    for i in 1..=3 {
        sqs.send_message(
            &queue_url,
            format!("job payload #{}", i),
            None,
            None,
            None,
            None,
        )
        .unwrap();
    }

    // 5. Run poller iteration
    let processed = state.poll_event_sources_once().await;
    assert_eq!(processed, 3);

    // 6. Verify SQS queue is now empty (messages were consumed and deleted by lambda poller)
    let remaining = sqs.receive_message(&queue_url, 10, None, None).await.unwrap();
    assert_eq!(remaining.len(), 0);
}
