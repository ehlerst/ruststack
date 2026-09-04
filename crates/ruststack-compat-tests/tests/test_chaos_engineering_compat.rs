use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_server::{create_router, AppState};
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tower::ServiceExt;

fn setup_test_app() -> axum::Router {
    let region = "us-east-1".to_string();
    let account_id = "000000000000".to_string();

    let s3_storage = Arc::new(ruststack_s3::InMemoryStorage::new());
    let sqs_engine = Arc::new(ruststack_sqs::SqsEngine::new(
        account_id.clone(),
        region.clone(),
    ));
    let sns_engine = Arc::new(ruststack_sns::SnsEngine::new(
        sqs_engine.clone(),
        account_id.clone(),
        region.clone(),
    ));
    let eventbridge_engine = Arc::new(ruststack_eventbridge::EventBridgeEngine::new(
        sqs_engine.clone(),
        sns_engine.clone(),
        account_id.clone(),
        region.clone(),
    ));
    let ssm_engine = Arc::new(ruststack_ssm::SsmEngine::new(
        account_id.clone(),
        region.clone(),
    ));
    let secretsmanager_engine = Arc::new(ruststack_secretsmanager::SecretsManagerEngine::new(
        account_id.clone(),
        region.clone(),
    ));
    let sts_engine = Arc::new(ruststack_sts::StsEngine::new(
        account_id.clone(),
        region.clone(),
    ));
    let dynamodb_engine = Arc::new(ruststack_dynamodb::DynamoDbEngine::new(
        account_id.clone(),
        region.clone(),
    ));
    let kms_state = Arc::new(ruststack_kms::KmsState::new(
        account_id.clone(),
        region.clone(),
    ));
    let logs_state = Arc::new(ruststack_logs::LogsState::new(
        account_id.clone(),
        region.clone(),
    ));
    let iam_state = Arc::new(ruststack_iam::IamState::new(account_id.clone()));
    let cloudwatch_state = Arc::new(ruststack_cloudwatch::CloudWatchState::new(
        account_id.clone(),
        region.clone(),
    ));
    let ses_state = Arc::new(ruststack_ses::SesState::new(
        account_id.clone(),
        region.clone(),
    ));
    let kinesis_state = Arc::new(ruststack_kinesis::KinesisState::new(
        account_id.clone(),
        region.clone(),
    ));
    let lambda_state = Arc::new(ruststack_lambda::LambdaState::new(
        account_id.clone(),
        region.clone(),
    ));
    let cognito_state = Arc::new(ruststack_cognito::CognitoState::new(
        account_id.clone(),
        region.clone(),
    ));
    let apigateway_state = Arc::new(ruststack_apigateway::ApiGatewayState::new(
        account_id.clone(),
        region.clone(),
    ));
    let route53_state = Arc::new(ruststack_route53::Route53State::new(
        account_id.clone(),
        region.clone(),
    ));
    let stepfunctions_state = Arc::new(ruststack_stepfunctions::StepFunctionsState::new(
        account_id.clone(),
        region.clone(),
    ));
    let cloudformation_state = Arc::new(ruststack_cloudformation::CloudFormationState::new(
        account_id.clone(),
        region.clone(),
    ));
    let ecr_state = Arc::new(ruststack_ecr::EcrState::new(
        account_id.clone(),
        region.clone(),
    ));
    let ecs_state = Arc::new(ruststack_ecs::EcsState::new(
        account_id.clone(),
        region.clone(),
    ));
    let ec2_state = Arc::new(ruststack_ec2::Ec2State::new(
        account_id.clone(),
        region.clone(),
    ));
    let elbv2_state = Arc::new(ruststack_elbv2::Elbv2State::new(
        account_id.clone(),
        region.clone(),
    ));
    let bedrock_state = Arc::new(ruststack_bedrock::BedrockState::new(
        account_id.clone(),
        region.clone(),
    ));
    let opensearch_state = Arc::new(ruststack_opensearch::OpenSearchState::new(
        account_id.clone(),
        region.clone(),
    ));
    let athena_state = Arc::new(ruststack_athena::AthenaState::new(
        account_id.clone(),
        region.clone(),
    ));
    let rds_state = Arc::new(ruststack_rds::RdsState::new(
        account_id.clone(),
        region.clone(),
    ));
    let elasticache_state = Arc::new(ruststack_elasticache::ElastiCacheState::new(
        account_id.clone(),
        region.clone(),
    ));
    let redshift_state = Arc::new(ruststack_redshift::RedshiftState::new(
        account_id.clone(),
        region.clone(),
    ));
    let acm_state = Arc::new(ruststack_acm::AcmState::new(
        account_id.clone(),
        region.clone(),
    ));
    let wafv2_state = Arc::new(ruststack_wafv2::Wafv2State::new(
        account_id.clone(),
        region.clone(),
    ));
    let organizations_state = Arc::new(ruststack_organizations::OrganizationsState::new(
        account_id.clone(),
        region.clone(),
    ));
    let chaos_engine = Arc::new(ruststack_core::ChaosEngine::new());

    let app_state = AppState {
        s3_storage,
        sqs_engine,
        sns_engine,
        eventbridge_engine,
        ssm_engine,
        secretsmanager_engine,
        sts_engine,
        dynamodb_engine,
        kms_state,
        logs_state,
        iam_state,
        cloudwatch_state,
        ses_state,
        kinesis_state,
        lambda_state,
        cognito_state,
        apigateway_state,
        route53_state,
        stepfunctions_state,
        cloudformation_state,
        ecr_state,
        ecs_state,
        ec2_state,
        elbv2_state,
        bedrock_state,
        opensearch_state,
        athena_state,
        rds_state,
        elasticache_state,
        redshift_state,
        acm_state,
        wafv2_state,
        organizations_state,
        chaos_engine,
        region,
        account_id,
    };

    create_router(app_state)
}

#[tokio::test]
async fn test_chaos_fault_injection_and_action_filtering() {
    let app = setup_test_app();

    // 1. Create a DynamoDB table
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.CreateTable")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "TableName": "ChaosTable",
                "KeySchema": [{ "AttributeName": "id", "KeyType": "HASH" }],
                "AttributeDefinitions": [{ "AttributeName": "id", "AttributeType": "S" }],
                "BillingMode": "PAY_PER_REQUEST"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Normal PutItem succeeds
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.PutItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "TableName": "ChaosTable",
                "Item": { "id": { "S": "record-1" } }
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Inject chaos rule on DynamoDB PutItem
    let req = Request::builder()
        .method(Method::POST)
        .uri("/_ruststack/chaos/rules")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "id": "dynamo-throttle-rule",
                "service": "dynamodb",
                "action": "PutItem",
                "probability": 1.0,
                "error_status": 400,
                "error_code": "ProvisionedThroughputExceededException",
                "error_message": "Rate of requests exceeds current capacity."
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 4. PutItem should now FAIL with ProvisionedThroughputExceededException
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.PutItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "TableName": "ChaosTable",
                "Item": { "id": { "S": "record-2" } }
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let err_val: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(err_val["__type"]
        .as_str()
        .unwrap()
        .contains("ProvisionedThroughputExceededException"));

    // 5. GetItem should NOT be affected (action-level isolation)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.GetItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "TableName": "ChaosTable",
                "Key": { "id": { "S": "record-1" } }
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. Disable chaos globally
    let req = Request::builder()
        .method(Method::POST)
        .uri("/_ruststack/chaos/disable")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 7. PutItem now succeeds while chaos is disabled
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.PutItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "TableName": "ChaosTable",
                "Item": { "id": { "S": "record-2" } }
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 8. Delete the rule
    let req = Request::builder()
        .method(Method::DELETE)
        .uri("/_ruststack/chaos/rules/dynamo-throttle-rule")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_chaos_s3_error_limit_times_and_latency_injection() {
    let app = setup_test_app();

    // 1. Create S3 Bucket and Object
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/chaos-bucket")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method(Method::PUT)
        .uri("/chaos-bucket/data.txt")
        .body(Body::from("payload"))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Inject S3 rule that fails twice with 503 SlowDown, then expires
    let req = Request::builder()
        .method(Method::POST)
        .uri("/_ruststack/chaos/rules")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "service": "s3",
                "probability": 1.0,
                "error_status": 503,
                "error_code": "SlowDown",
                "error_message": "Please reduce your request rate.",
                "limit_times": 2
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 1st request -> 503
    let req = Request::builder()
        .method(Method::GET)
        .uri("/chaos-bucket/data.txt")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    // 2nd request -> 503
    let req = Request::builder()
        .method(Method::GET)
        .uri("/chaos-bucket/data.txt")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    // 3rd request -> SUCCESS (200 OK) because rule expired!
    let req = Request::builder()
        .method(Method::GET)
        .uri("/chaos-bucket/data.txt")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(bytes, "payload");

    // 3. Test SQS Latency Injection
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.CreateQueue")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({ "QueueName": "delay-queue" }).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let q_val: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let q_url = q_val["QueueUrl"].as_str().unwrap();

    // Inject 50ms latency on SQS SendMessage
    let req = Request::builder()
        .method(Method::POST)
        .uri("/_ruststack/chaos/rules")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "service": "sqs",
                "action": "SendMessage",
                "latency_ms": 60,
                "latency_jitter_ms": 10
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let start = Instant::now();
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.SendMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({ "QueueUrl": q_url, "MessageBody": "delayed-msg" }).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let elapsed = start.elapsed();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(
        elapsed.as_millis() >= 45,
        "Expected injected latency >= 45ms, took {}ms",
        elapsed.as_millis()
    );
}
