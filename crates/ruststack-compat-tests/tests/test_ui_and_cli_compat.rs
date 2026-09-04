use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_server::{create_router, AppState};
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
async fn test_embedded_ui_delivery() {
    let app = setup_test_app();

    // 1. Direct /_ruststack/ui endpoint
    let req = Request::builder()
        .method(Method::GET)
        .uri("/_ruststack/ui")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&body_bytes);
    assert!(html.contains("RustStack Cloud Console"));
    assert!(html.contains("Chaos Studio"));
    assert!(html.contains("Snapshots"));

    // 2. Direct /_ruststack/ui/ endpoint
    let req = Request::builder()
        .method(Method::GET)
        .uri("/_ruststack/ui/")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Browser root request with Accept: text/html
    let req = Request::builder()
        .method(Method::GET)
        .uri("/")
        .header("accept", "text/html,application/xhtml+xml")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&body_bytes);
    assert!(html.contains("RustStack Cloud Console"));
}
