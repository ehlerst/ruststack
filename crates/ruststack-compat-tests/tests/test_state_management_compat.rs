use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_server::state_api::RustStackStateSnapshot;
use ruststack_server::{create_router, AppState};
use serde_json::json;
use std::sync::Arc;
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
        chaos_engine: Arc::new(ruststack_core::ChaosEngine::new()),
        region,
        account_id,
    };

    create_router(app_state)
}

#[tokio::test]
async fn test_state_management_full_lifecycle_and_selective_reset() {
    let app = setup_test_app();

    // 1. Populate S3
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/my-snapshot-bucket")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method(Method::PUT)
        .uri("/my-snapshot-bucket/file.txt")
        .body(Body::from("snapshot-content-s3"))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Populate SQS
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.CreateQueue")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({ "QueueName": "snapshot-queue" }).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let q_val: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let q_url = q_val["QueueUrl"].as_str().unwrap();

    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.SendMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({ "QueueUrl": q_url, "MessageBody": "msg-in-queue-snap" }).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Populate SSM
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSSM.PutParameter")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "Name": "/app/config/db_host",
                "Value": "db.snapshot.internal",
                "Type": "String"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. Populate DynamoDB
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.CreateTable")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "TableName": "SnapTable",
                "KeySchema": [{ "AttributeName": "id", "KeyType": "HASH" }],
                "AttributeDefinitions": [{ "AttributeName": "id", "AttributeType": "S" }],
                "BillingMode": "PAY_PER_REQUEST"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.PutItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "TableName": "SnapTable",
                "Item": {
                    "id": { "S": "record-1" },
                    "value": { "S": "preserved-across-snapshots" }
                }
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. Populate SNS & EventBridge
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSNS.CreateTopic")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(json!({ "Name": "snapshot-topic" }).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AWSEvents.PutRule")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from(
            json!({
                "Name": "snapshot-rule",
                "EventPattern": "{\"source\":[\"custom.snap\"]}"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. DUMP STATE (GET /_ruststack/state/dump)
    let req = Request::builder()
        .method(Method::GET)
        .uri("/_ruststack/state/dump")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let dump_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let snapshot: RustStackStateSnapshot = serde_json::from_slice(&dump_bytes).unwrap();

    assert_eq!(snapshot.version, 1);
    assert_eq!(snapshot.s3.buckets.len(), 1);
    assert_eq!(snapshot.s3.buckets[0].info.name, "my-snapshot-bucket");
    assert_eq!(snapshot.s3.buckets[0].objects.len(), 1);
    assert_eq!(snapshot.sqs.queues.len(), 1);
    assert_eq!(snapshot.ssm.parameters.len(), 1);
    assert_eq!(snapshot.dynamodb.tables.len(), 1);
    assert_eq!(snapshot.dynamodb.tables[0].items.len(), 1);
    assert_eq!(snapshot.sns.topics.len(), 1);
    assert_eq!(snapshot.eventbridge.rules.len(), 1);

    // 7. TEST SELECTIVE RESET (reset only S3)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/_ruststack/state/reset")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "services": ["s3"] }).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // S3 should be empty
    let req = Request::builder()
        .method(Method::GET)
        .uri("/my-snapshot-bucket/file.txt")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // But DynamoDB should still have data!
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.GetItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "TableName": "SnapTable",
                "Key": { "id": { "S": "record-1" } }
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 8. RESET ALL STATE (POST /_ruststack/state/reset)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/_ruststack/state/reset")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify DynamoDB is now empty
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.GetItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "TableName": "SnapTable",
                "Key": { "id": { "S": "record-1" } }
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // 9. RESTORE FROM SNAPSHOT (POST /_ruststack/state/load)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/_ruststack/state/load")
        .header("content-type", "application/json")
        .body(Body::from(dump_bytes))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 10. Verify S3 is restored
    let req = Request::builder()
        .method(Method::GET)
        .uri("/my-snapshot-bucket/file.txt")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let s3_text = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(s3_text, "snapshot-content-s3");

    // 11. Verify SQS is restored
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSQS.ReceiveMessage")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({ "QueueUrl": q_url, "MaxNumberOfMessages": 1 }).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let sqs_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let sqs_val: serde_json::Value = serde_json::from_slice(&sqs_bytes).unwrap();
    assert_eq!(
        sqs_val["Messages"][0]["Body"].as_str().unwrap(),
        "msg-in-queue-snap"
    );

    // 12. Verify DynamoDB is restored
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.GetItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            json!({
                "TableName": "SnapTable",
                "Key": { "id": { "S": "record-1" } }
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ddb_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let ddb_val: serde_json::Value = serde_json::from_slice(&ddb_bytes).unwrap();
    assert_eq!(
        ddb_val["Item"]["value"]["S"].as_str().unwrap(),
        "preserved-across-snapshots"
    );

    // 13. Verify SNS is restored
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AmazonSNS.ListTopics")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from("{}"))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let sns_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let sns_val: serde_json::Value = serde_json::from_slice(&sns_bytes).unwrap();
    assert_eq!(sns_val["Topics"].as_array().unwrap().len(), 1);

    // 14. Verify EventBridge is restored
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "AWSEvents.ListRules")
        .header("content-type", "application/x-amz-json-1.1")
        .body(Body::from("{}"))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let eb_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let eb_val: serde_json::Value = serde_json::from_slice(&eb_bytes).unwrap();
    assert_eq!(eb_val["Rules"].as_array().unwrap().len(), 1);
}
