use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_dynamodb::DynamoDbEngine;
use ruststack_eventbridge::EventBridgeEngine;
use ruststack_s3::InMemoryStorage;
use ruststack_secretsmanager::SecretsManagerEngine;
use ruststack_server::{create_router, AppState};
use ruststack_sns::SnsEngine;
use ruststack_sqs::SqsEngine;
use ruststack_ssm::SsmEngine;
use ruststack_sts::StsEngine;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn test_server_unified_routing() {
    let s3_storage = Arc::new(InMemoryStorage::new());
    let sqs_engine = Arc::new(SqsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let sns_engine = Arc::new(SnsEngine::new(
        sqs_engine.clone(),
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let eventbridge_engine = Arc::new(EventBridgeEngine::new(
        sqs_engine.clone(),
        sns_engine.clone(),
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let ssm_engine = Arc::new(SsmEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let secretsmanager_engine = Arc::new(SecretsManagerEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let sts_engine = Arc::new(StsEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let dynamodb_engine = Arc::new(DynamoDbEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let kms_state = Arc::new(ruststack_kms::KmsState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let logs_state = Arc::new(ruststack_logs::LogsState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let iam_state = Arc::new(ruststack_iam::IamState::new("000000000000".to_string()));
    let cloudwatch_state = Arc::new(ruststack_cloudwatch::CloudWatchState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let ses_state = Arc::new(ruststack_ses::SesState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let kinesis_state = Arc::new(ruststack_kinesis::KinesisState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let lambda_state = Arc::new(ruststack_lambda::LambdaState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let cognito_state = Arc::new(ruststack_cognito::CognitoState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let apigateway_state = Arc::new(ruststack_apigateway::ApiGatewayState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let route53_state = Arc::new(ruststack_route53::Route53State::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let stepfunctions_state = Arc::new(ruststack_stepfunctions::StepFunctionsState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let cloudformation_state = Arc::new(ruststack_cloudformation::CloudFormationState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let ecr_state = Arc::new(ruststack_ecr::EcrState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let ecs_state = Arc::new(ruststack_ecs::EcsState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let ec2_state = Arc::new(ruststack_ec2::Ec2State::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let elbv2_state = Arc::new(ruststack_elbv2::Elbv2State::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let bedrock_state = Arc::new(ruststack_bedrock::BedrockState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let opensearch_state = Arc::new(ruststack_opensearch::OpenSearchState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let athena_state = Arc::new(ruststack_athena::AthenaState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let rds_state = Arc::new(ruststack_rds::RdsState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let elasticache_state = Arc::new(ruststack_elasticache::ElastiCacheState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let redshift_state = Arc::new(ruststack_redshift::RedshiftState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let acm_state = Arc::new(ruststack_acm::AcmState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let wafv2_state = Arc::new(ruststack_wafv2::Wafv2State::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));
    let organizations_state = Arc::new(ruststack_organizations::OrganizationsState::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ));

    let state = AppState {
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
        region: "us-east-1".to_string(),
        account_id: "000000000000".to_string(),
    };

    let app = create_router(state);

    // 1. Health check
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/_ruststack/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(body.as_ref().starts_with(b"{\"status\": \"running\""));

    // 2. S3 create bucket via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/unified-bucket")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. SQS create queue via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AmazonSQS.CreateQueue")
                .header("content-type", "application/x-amz-json-1.0")
                .body(Body::from(r#"{"QueueName": "unified-queue"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. SNS create topic via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AmazonSNS.CreateTopic")
                .header("content-type", "application/x-amz-json-1.0")
                .body(Body::from(r#"{"Name": "unified-topic"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. EventBridge PutEvents via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AWSEvents.PutEvents")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(
                    serde_json::json!({
                        "Entries": [
                            {
                                "Source": "unified.test",
                                "DetailType": "HealthEvent",
                                "Detail": "{\"status\": \"ok\"}"
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. SSM PutParameter via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AmazonSSM.PutParameter")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(
                    serde_json::json!({
                        "Name": "/test/key",
                        "Value": "val123",
                        "Type": "String"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 7. SecretsManager CreateSecret via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "secretsmanager.CreateSecret")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(
                    serde_json::json!({
                        "Name": "test-secret",
                        "SecretString": "sec123"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 8. STS GetCallerIdentity via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header(
                    "x-amz-target",
                    "AWSSecurityTokenServiceV20110615.GetCallerIdentity",
                )
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 9. DynamoDB CreateTable via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "DynamoDB_20120810.CreateTable")
                .header("content-type", "application/x-amz-json-1.0")
                .body(Body::from(
                    serde_json::json!({
                        "TableName": "UnifiedTable",
                        "KeySchema": [
                            { "AttributeName": "pk", "KeyType": "HASH" }
                        ],
                        "AttributeDefinitions": [
                            { "AttributeName": "pk", "AttributeType": "S" }
                        ],
                        "BillingMode": "PAY_PER_REQUEST"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        val["TableDescription"]["TableStatus"].as_str().unwrap(),
        "ACTIVE"
    );

    // 10. KMS CreateKey via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "TrentService.CreateKey")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(r#"{"Description": "unified-kms-key"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 11. CloudWatch Logs CreateLogGroup via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "Logs_20140328.CreateLogGroup")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(r#"{"logGroupName": "/aws/lambda/unified-log"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 12. IAM CreateRole via unified router (Query protocol)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "Action=CreateRole&RoleName=UnifiedRole&AssumeRolePolicyDocument=%7B%7D",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 13. CloudWatch PutMetricData via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("Action=PutMetricData&Namespace=System&MetricData.member.1.MetricName=CPUUtilization&MetricData.member.1.Value=12.5"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 14. SES VerifyEmailIdentity via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "Action=VerifyEmailIdentity&EmailAddress=admin@example.com",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 15. Kinesis CreateStream via unified router
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "Kinesis_20131202.CreateStream")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(
                    r#"{"StreamName": "unified-stream", "ShardCount": 1}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 16. Lambda CreateFunction via unified router (REST JSON)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/2015-03-31/functions")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "FunctionName": "unified-func",
                        "Runtime": "python3.11",
                        "Role": "arn:aws:iam::000000000000:role/service-role",
                        "Handler": "index.handler"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 17. Cognito CreateUserPool via unified router (JSON target)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AWSCognitoIdentityProviderService.CreateUserPool")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(
                    serde_json::json!({
                        "PoolName": "unified-pool"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 18. API Gateway CreateRestApi via unified router (REST JSON)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/restapis")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "name": "unified-api"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 19. Route53 CreateHostedZone via unified router (REST JSON)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/2013-04-01/hostedzone")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "Name": "example.internal.",
                        "CallerReference": "ref-unified-123"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 20. StepFunctions CreateStateMachine via unified router (JSON 1.0 target)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AWSStepFunctions.CreateStateMachine")
                .header("content-type", "application/x-amz-json-1.0")
                .body(Body::from(
                    serde_json::json!({
                        "Name": "unified-state-machine",
                        "Definition": "{\"StartAt\":\"PassState\",\"States\":{\"PassState\":{\"Type\":\"Pass\",\"End\":true}}}",
                        "RoleArn": "arn:aws:iam::000000000000:role/SFNRole"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 21. CloudFormation CreateStack via unified router (Query Protocol)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "Action=CreateStack&StackName=unified-stack&TemplateBody=%7B%22Resources%22%3A%7B%7D%7D",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 22. ECR CreateRepository via unified router (JSON 1.1 target)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AmazonEC2ContainerRegistry_V20150921.CreateRepository")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(
                    serde_json::json!({
                        "repositoryName": "unified-repo"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 23. ECS CreateCluster via unified router (JSON 1.1 target)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AmazonEC2ContainerServiceV20141113.CreateCluster")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(
                    serde_json::json!({
                        "clusterName": "unified-ecs-cluster"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 24. EC2 CreateVpc via unified router (Query Protocol)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "Action=CreateVpc&CidrBlock=192.168.0.0/16",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 25. ELBv2 CreateTargetGroup via unified router (Query Protocol)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "Action=CreateTargetGroup&Name=unified-tg&Protocol=HTTP&Port=8080",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 26. Bedrock ListFoundationModels via unified router (REST)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/foundation-models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 27. OpenSearch CreateDomain via unified router (REST)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/2021-01-01/opensearch/domain")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "DomainName": "unified-domain"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 28. Athena StartQueryExecution via unified router (JSON 1.1)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AmazonAthena.StartQueryExecution")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(
                    serde_json::json!({
                        "QueryString": "SELECT 1"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 29. RDS CreateDBInstance via unified router (Query Protocol)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "Action=CreateDBInstance&DBInstanceIdentifier=router-db&Engine=postgres&DBInstanceClass=db.t3.micro&MasterUsername=root",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 30. ElastiCache CreateCacheCluster via unified router (Query Protocol)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "Action=CreateCacheCluster&CacheClusterIdentifier=router-cache&Engine=redis",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 31. Redshift CreateCluster via unified router (Query Protocol)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "Action=CreateCluster&ClusterIdentifier=router-redshift&NodeType=dc2.large&MasterUsername=admin&DBName=analytics",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 32. ACM RequestCertificate via unified router (JSON 1.1)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "CertificateManager.RequestCertificate")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(
                    serde_json::json!({
                        "DomainName": "unified.example.com",
                        "ValidationMethod": "DNS"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 33. WAFv2 CreateWebACL via unified router (JSON 1.1)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AWSWAF_20190729.CreateWebACL")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from(
                    serde_json::json!({
                        "Name": "router-waf",
                        "Scope": "REGIONAL",
                        "DefaultAction": { "Allow": {} },
                        "VisibilityConfig": {
                            "SampledRequestsEnabled": true,
                            "CloudWatchMetricsEnabled": true,
                            "MetricName": "RouterWafMetric"
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 34. Organizations CreateOrganization via unified router (JSON 1.1)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("x-amz-target", "AWSOrganizationsV20161128.CreateOrganization")
                .header("content-type", "application/x-amz-json-1.1")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
