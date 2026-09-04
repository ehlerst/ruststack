use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{any, delete, get};
use axum::Router;
use clap::Parser;
use http_body_util::BodyExt;
use ruststack_acm::{handle_acm_request, AcmState};
use ruststack_apigateway::{handle_apigateway_request, ApiGatewayState};
use ruststack_athena::{handle_athena_request, AthenaState};
use ruststack_bedrock::{handle_bedrock_request, BedrockState};
use ruststack_cloudformation::{handle_cloudformation_request, CloudFormationState};
use ruststack_cloudwatch::{handle_cloudwatch_request, CloudWatchState};
use ruststack_cognito::{handle_cognito_request, CognitoState};
use ruststack_core::{AwsService, ChaosDecision, ChaosEngine, Dispatcher};
use ruststack_dynamodb::{handle_dynamodb_request, DynamoDbEngine};
use ruststack_ec2::{handle_ec2_request, Ec2State};
use ruststack_ecr::{handle_ecr_request, EcrState};
use ruststack_ecs::{handle_ecs_request, EcsState};
use ruststack_elasticache::{handle_elasticache_request, ElastiCacheState};
use ruststack_elbv2::{handle_elbv2_request, Elbv2State};
use ruststack_eventbridge::{handle_eventbridge_request, EventBridgeEngine};
use ruststack_iam::{handle_iam_request, IamState};
use ruststack_kinesis::{handle_kinesis_request, KinesisState};
use ruststack_kms::{handle_kms_request, KmsState};
use ruststack_lambda::{handle_lambda_request, LambdaState};
use ruststack_logs::{handle_logs_request, LogsState};
use ruststack_opensearch::{handle_opensearch_request, OpenSearchState};
use ruststack_organizations::{handle_organizations_request, OrganizationsState};
use ruststack_rds::{handle_rds_request, RdsState};
use ruststack_redshift::{handle_redshift_request, RedshiftState};
use ruststack_route53::{handle_route53_request, Route53State};
use ruststack_s3::{handle_s3_request, S3NotificationTarget, S3Storage};
use ruststack_secretsmanager::{handle_secretsmanager_request, SecretsManagerEngine};
use ruststack_ses::{handle_ses_request, SesState};
use ruststack_sns::{handle_sns_request, SnsEngine};
use ruststack_sqs::{handle_sqs_request, SqsEngine};
use ruststack_ssm::{handle_ssm_request, SsmEngine};
use ruststack_stepfunctions::{handle_stepfunctions_request, StepFunctionsState};
use ruststack_sts::{handle_sts_request, StsEngine};
use ruststack_wafv2::{handle_wafv2_request, Wafv2State};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::chaos_api::{
    add_chaos_rule_handler, clear_chaos_rules_handler, delete_chaos_rule_handler,
    disable_chaos_handler, enable_chaos_handler, list_chaos_rules_handler,
};
use crate::state_api::{state_dump_handler, state_load_handler, state_reset_handler};
use crate::ui::serve_ui;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "ruststack",
    about = "Blazing-fast local AWS cloud emulator in Rust"
)]
pub struct Opts {
    #[arg(short, long, default_value = "4566", env = "PORT")]
    pub port: u16,

    #[arg(long, default_value = "0.0.0.0", env = "HOST")]
    pub host: String,

    #[arg(
        short,
        long,
        default_value = "s3,sqs,sns,events,ssm,secretsmanager,sts,dynamodb,kms,logs,iam,cloudwatch,ses,kinesis,lambda,cognito,apigateway,route53,stepfunctions,cloudformation,ecr,ecs,ec2,elbv2,bedrock,opensearch,athena,rds,elasticache,redshift",
        env = "SERVICES"
    )]
    pub services: String,

    #[arg(long, default_value = "us-east-1", env = "DEFAULT_REGION")]
    pub region: String,

    #[arg(long, default_value = "000000000000", env = "ACCOUNT_ID")]
    pub account_id: String,

    #[arg(long, env = "RUSTSTACK_DATA_DIR", alias = "data-dir")]
    pub data_dir: Option<String>,

    #[arg(long, env = "PERSISTENCE", default_value_t = false)]
    pub persistence: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            port: 4566,
            host: "0.0.0.0".to_string(),
            services: "s3,sqs,sns,events,ssm,secretsmanager,sts,dynamodb,kms,logs,iam,cloudwatch,ses,kinesis,lambda,cognito,apigateway,route53,stepfunctions,cloudformation,ecr,ecs,ec2,elbv2,bedrock,opensearch,athena,rds,elasticache,redshift,acm,wafv2,organizations".to_string(),
            region: "us-east-1".to_string(),
            account_id: "000000000000".to_string(),
            data_dir: None,
            persistence: false,
        }
    }
}

pub struct ServerNotificationTarget {
    sqs: Arc<SqsEngine>,
    sns: Arc<SnsEngine>,
    eventbridge: Arc<EventBridgeEngine>,
}

impl ServerNotificationTarget {
    pub fn new(
        sqs: Arc<SqsEngine>,
        sns: Arc<SnsEngine>,
        eventbridge: Arc<EventBridgeEngine>,
    ) -> Self {
        Self {
            sqs,
            sns,
            eventbridge,
        }
    }
}

impl S3NotificationTarget for ServerNotificationTarget {
    fn send_sqs(&self, queue_arn: &str, payload: &str) {
        let _ = self
            .sqs
            .send_message(queue_arn, payload.to_string(), None, None, None, None);
    }

    fn send_sns(&self, topic_arn: &str, payload: &str) {
        let _ = self
            .sns
            .publish(topic_arn, payload.to_string(), None, None, None, None);
    }

    fn send_eventbridge(&self, source: &str, detail_type: &str, detail: &str) {
        let _ = self.eventbridge.put_events(vec![
            ruststack_eventbridge::types::PutEventsRequestEntry {
                event_bus_name: Some("default".to_string()),
                source: Some(source.to_string()),
                detail_type: Some(detail_type.to_string()),
                detail: Some(detail.to_string()),
                resources: None,
                time: None,
                trace_header: None,
            },
        ]);
    }
}

#[derive(Clone)]
pub struct AppState {
    pub s3_storage: Arc<dyn S3Storage>,
    pub sqs_engine: Arc<SqsEngine>,
    pub sns_engine: Arc<SnsEngine>,
    pub eventbridge_engine: Arc<EventBridgeEngine>,
    pub ssm_engine: Arc<SsmEngine>,
    pub secretsmanager_engine: Arc<SecretsManagerEngine>,
    pub sts_engine: Arc<StsEngine>,
    pub dynamodb_engine: Arc<DynamoDbEngine>,
    pub kms_state: Arc<KmsState>,
    pub logs_state: Arc<LogsState>,
    pub iam_state: Arc<IamState>,
    pub cloudwatch_state: Arc<CloudWatchState>,
    pub ses_state: Arc<SesState>,
    pub kinesis_state: Arc<KinesisState>,
    pub lambda_state: Arc<LambdaState>,
    pub cognito_state: Arc<CognitoState>,
    pub apigateway_state: Arc<ApiGatewayState>,
    pub route53_state: Arc<Route53State>,
    pub stepfunctions_state: Arc<StepFunctionsState>,
    pub cloudformation_state: Arc<CloudFormationState>,
    pub ecr_state: Arc<EcrState>,
    pub ecs_state: Arc<EcsState>,
    pub ec2_state: Arc<Ec2State>,
    pub elbv2_state: Arc<Elbv2State>,
    pub bedrock_state: Arc<BedrockState>,
    pub opensearch_state: Arc<OpenSearchState>,
    pub athena_state: Arc<AthenaState>,
    pub rds_state: Arc<RdsState>,
    pub elasticache_state: Arc<ElastiCacheState>,
    pub redshift_state: Arc<RedshiftState>,
    pub acm_state: Arc<AcmState>,
    pub wafv2_state: Arc<Wafv2State>,
    pub organizations_state: Arc<OrganizationsState>,
    pub chaos_engine: Arc<ChaosEngine>,
    pub region: String,
    pub account_id: String,
}

pub fn create_router(state: AppState) -> Router {
    state
        .s3_storage
        .set_notification_target(Arc::new(ServerNotificationTarget::new(
            state.sqs_engine.clone(),
            state.sns_engine.clone(),
            state.eventbridge_engine.clone(),
        )));

    state.lambda_state.set_sqs_engine(state.sqs_engine.clone());
    state
        .cloudwatch_state
        .set_sns_engine(state.sns_engine.clone());
    state
        .apigateway_state
        .set_lambda_state(state.lambda_state.clone());
    state
        .stepfunctions_state
        .set_lambda_state(state.lambda_state.clone());
    state.cloudformation_state.set_services(
        Some(state.s3_storage.clone()),
        Some(state.sqs_engine.clone()),
        Some(state.sns_engine.clone()),
        Some(state.dynamodb_engine.clone()),
        Some(state.iam_state.clone()),
    );

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any);

    Router::new()
        .route("/_ruststack/ui", get(serve_ui))
        .route("/_ruststack/ui/", get(serve_ui))
        .route("/_ruststack/health", get(health_check))
        .route("/_ruststack/info", get(info_handler))
        .route("/_ruststack/state/reset", any(state_reset_handler))
        .route("/_ruststack/state/dump", any(state_dump_handler))
        .route("/_ruststack/state/load", any(state_load_handler))
        .route(
            "/_ruststack/chaos/rules",
            get(list_chaos_rules_handler)
                .post(add_chaos_rule_handler)
                .delete(clear_chaos_rules_handler),
        )
        .route(
            "/_ruststack/chaos/rules/{id}",
            delete(delete_chaos_rule_handler),
        )
        .route("/_ruststack/chaos/reset", any(clear_chaos_rules_handler))
        .route("/_ruststack/chaos/enable", any(enable_chaos_handler))
        .route("/_ruststack/chaos/disable", any(disable_chaos_handler))
        .route("/health", get(health_check))
        .fallback(any(gateway_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        "{\"status\": \"running\", \"engine\": \"ruststack\"}",
    )
}

async fn info_handler(State(state): State<AppState>) -> impl IntoResponse {
    let json_info = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "services": [
            "s3", "sqs", "sns", "events", "ssm", "secretsmanager", "sts",
            "dynamodb", "kms", "logs", "iam", "cloudwatch", "ses", "kinesis", "lambda", "dynamodbstreams",
            "cognito", "apigateway", "route53", "stepfunctions", "cloudformation", "ecr",
            "ecs", "ec2", "elbv2", "bedrock", "opensearch", "athena", "rds", "elasticache", "redshift",
            "acm", "wafv2", "organizations"
        ],
        "region": state.region,
        "account_id": state.account_id,
        "features": {
            "s3": ["buckets", "objects", "multipart", "byte-range", "virtual-hosting", "bucket-notifications", "versioning", "lifecycle", "cors", "policy", "tagging"],
            "sqs": ["standard-queue", "fifo-queue", "dlq-redrive", "long-polling", "batching", "visibility-timeout", "query-and-json-protocols", "tagging", "delays"],
            "sns": ["topics", "subscriptions", "sqs-fanout", "filter-policy", "raw-message-delivery", "http-webhooks", "query-and-json-protocols"],
            "events": ["event-buses", "rules", "pattern-matching", "targets-sqs-sns", "json-protocol"],
            "ssm": ["parameters", "hierarchical-paths", "versioning", "secure-string", "json-protocol"],
            "secretsmanager": ["secrets", "version-stages", "rotation", "binary-and-string", "json-protocol"],
            "sts": ["caller-identity", "assume-role", "session-tokens", "query-and-json-protocols"],
            "dynamodb": ["tables", "crud", "query", "scan", "key-conditions", "filter-expressions", "gsi-lsi", "batching", "transactions", "ttl", "json-protocol"],
            "dynamodbstreams": ["shards", "shard-iterators", "get-records", "insert-modify-remove-capture"],
            "kms": ["keys", "aliases", "encrypt-decrypt", "generate-data-key", "json-protocol"],
            "logs": ["log-groups", "log-streams", "put-events", "filter-events", "json-protocol"],
            "iam": ["roles", "policies", "users", "access-keys", "query-and-json-protocols"],
            "cloudwatch": ["put-metric-data", "get-metric-data", "get-metric-statistics", "alarms", "alarm-actions", "query-and-json-protocols"],
            "ses": ["send-email", "send-raw-email", "verify-identities", "templates", "send-templated-email", "quota", "query-and-json-protocols"],
            "kinesis": ["streams", "shards", "put-record", "put-records", "shard-iterators", "get-records", "resharding", "json-protocol"],
            "lambda": ["functions", "synchronous-invoke", "asynchronous-invoke", "event-source-mappings", "sqs-poller", "rest-and-json-protocols"],
            "cognito": ["user-pools", "user-pool-clients", "sign-up", "admin-create-user", "auth", "jwt-tokens", "jwks", "json-protocol"],
            "apigateway": ["rest-apis", "resources", "methods", "mock-integrations", "lambda-integrations", "deployments", "stages", "invocation-routing"],
            "route53": ["hosted-zones", "change-resource-record-sets", "list-resource-record-sets", "xml-and-json-protocols"],
            "stepfunctions": ["state-machines", "asl-execution-engine", "pass-task-choice-wait-succeed-fail", "execution-history", "json-protocol"],
            "cloudformation": ["stack-lifecycle", "template-parser", "resource-orchestration", "outputs", "xml-and-json-protocols"],
            "ecr": ["repositories", "image-manifests", "tags", "digests", "auth-tokens", "json-protocol"],
            "ecs": ["clusters", "task-definitions", "tasks", "services", "fargate-launch-type", "json-protocol"],
            "ec2": ["vpcs", "subnets", "security-groups", "ingress-rules", "key-pairs", "instances", "xml-and-query-protocols"],
            "elbv2": ["application-load-balancers", "target-groups", "target-registration", "listeners", "health-checks", "xml-and-query-protocols"],
            "bedrock": ["foundation-models", "claude-llama-titan-models", "model-invocation", "streaming-token-responses", "rest-and-json-protocols"],
            "opensearch": ["domains", "cluster-configurations", "search-endpoints", "encryption", "rest-and-json-protocols"],
            "athena": ["query-execution", "named-queries", "workgroups", "result-sets", "json-protocol"],
            "rds": ["db-instances", "db-clusters", "snapshots", "postgres-mysql-aurora", "xml-and-query-protocols"],
            "elasticache": ["cache-clusters", "replication-groups", "redis-memcached", "xml-and-query-protocols"],
            "redshift": ["clusters", "snapshots", "data-warehousing", "xml-and-query-protocols"],
            "acm": ["request-certificate", "describe-certificate", "list-certificates", "x509-mocking", "dns-validation", "json-protocol"],
            "wafv2": ["web-acls", "rules", "ip-sets", "alb-and-apigateway-associations", "json-protocol"],
            "organizations": ["organizations", "roots", "accounts", "organizational-units", "json-protocol"],
            "chaos": ["latency-injection", "jitter", "error-rate-simulation", "rule-limits", "service-filtering"]
        }
    });

    (
        StatusCode::OK,
        [("content-type", "application/json")],
        json_info.to_string(),
    )
}

async fn gateway_handler(State(state): State<AppState>, req: Request<Body>) -> Response<Body> {
    let (parts, body) = req.into_parts();
    let method = parts.method.clone();
    let uri = parts.uri.clone();
    let headers = parts.headers.clone();

    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(e.to_string()))
                .unwrap();
        }
    };

    // Browser auto-detection on root path
    if (uri.path() == "/" || uri.path() == "")
        && method == axum::http::Method::GET
        && headers
            .get("accept")
            .and_then(|a| a.to_str().ok())
            .unwrap_or("")
            .contains("text/html")
        && !headers.contains_key("authorization")
        && !headers.contains_key("x-amz-date")
    {
        return Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html; charset=utf-8")
            .body(Body::from(crate::ui::EMBEDDED_UI_HTML))
            .unwrap();
    }

    // Check service classification with body peek for Form Query actions
    let service = Dispatcher::classify_request(&method, &uri, &headers, Some(&body_bytes));

    // Determine action for chaos rule matching
    let action_opt: Option<String> = if let Some(target) = headers.get("x-amz-target") {
        if let Ok(target_str) = target.to_str() {
            if let Some(pos) = target_str.rfind('.') {
                Some(target_str[pos + 1..].to_string())
            } else {
                Some(target_str.to_string())
            }
        } else {
            None
        }
    } else if let Some(query) = uri.query() {
        form_urlencoded::parse(query.as_bytes())
            .find(|(k, _)| k.eq_ignore_ascii_case("Action"))
            .map(|(_, v)| v.into_owned())
    } else if let Some(content_type) = headers.get("content-type") {
        if content_type
            .to_str()
            .unwrap_or("")
            .starts_with("application/x-www-form-urlencoded")
        {
            form_urlencoded::parse(&body_bytes)
                .find(|(k, _)| k.eq_ignore_ascii_case("Action"))
                .map(|(_, v)| v.into_owned())
        } else {
            None
        }
    } else {
        None
    };

    // Evaluate chaos rules
    let decision = state
        .chaos_engine
        .evaluate(service, action_opt.as_deref(), uri.path());

    match decision {
        ChaosDecision::PassThrough { latency_ms } => {
            if let Some(ms) = latency_ms {
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            }
        }
        ChaosDecision::InjectError {
            status,
            code,
            message,
            latency_ms,
        } => {
            if let Some(ms) = latency_ms {
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            }
            let request_id = uuid::Uuid::new_v4().to_string();
            return ChaosEngine::format_error_response(
                service,
                status,
                &code,
                &message,
                &request_id,
            );
        }
    }

    let reconstructed_req = Request::from_parts(parts, Body::from(body_bytes.clone()));

    match service {
        AwsService::S3 => handle_s3_request(state.s3_storage.clone(), reconstructed_req).await,
        AwsService::Sqs => handle_sqs_request(state.sqs_engine.clone(), reconstructed_req).await,
        AwsService::Sns => handle_sns_request(state.sns_engine.clone(), reconstructed_req).await,
        AwsService::EventBridge => {
            handle_eventbridge_request(state.eventbridge_engine.clone(), reconstructed_req).await
        }
        AwsService::Ssm => handle_ssm_request(state.ssm_engine.clone(), reconstructed_req).await,
        AwsService::SecretsManager => {
            handle_secretsmanager_request(state.secretsmanager_engine.clone(), reconstructed_req)
                .await
        }
        AwsService::Sts => handle_sts_request(state.sts_engine.clone(), reconstructed_req).await,
        AwsService::DynamoDb => {
            handle_dynamodb_request(state.dynamodb_engine.clone(), reconstructed_req).await
        }
        AwsService::Kms => {
            handle_kms_request(State((*state.kms_state).clone()), headers, body_bytes).await
        }
        AwsService::Logs => {
            handle_logs_request(State((*state.logs_state).clone()), headers, body_bytes).await
        }
        AwsService::Iam => {
            handle_iam_request(State((*state.iam_state).clone()), uri, headers, body_bytes).await
        }
        AwsService::CloudWatch => {
            handle_cloudwatch_request(
                State((*state.cloudwatch_state).clone()),
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Ses => {
            handle_ses_request(State((*state.ses_state).clone()), uri, headers, body_bytes).await
        }
        AwsService::Kinesis => {
            handle_kinesis_request(State((*state.kinesis_state).clone()), headers, body_bytes).await
        }
        AwsService::Lambda => {
            handle_lambda_request(
                State((*state.lambda_state).clone()),
                method,
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Cognito => {
            handle_cognito_request(
                State((*state.cognito_state).clone()),
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::ApiGateway => {
            handle_apigateway_request(
                State((*state.apigateway_state).clone()),
                method,
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Route53 => {
            handle_route53_request(
                State((*state.route53_state).clone()),
                method,
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::StepFunctions => {
            handle_stepfunctions_request(
                State((*state.stepfunctions_state).clone()),
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::CloudFormation => {
            handle_cloudformation_request(
                State((*state.cloudformation_state).clone()),
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Ecr => {
            handle_ecr_request(
                State((*state.ecr_state).clone()),
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Ecs => {
            handle_ecs_request(
                State((*state.ecs_state).clone()),
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Ec2 => {
            handle_ec2_request(
                State((*state.ec2_state).clone()),
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Elbv2 => {
            handle_elbv2_request(
                State((*state.elbv2_state).clone()),
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Bedrock => {
            handle_bedrock_request(
                State((*state.bedrock_state).clone()),
                method,
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::OpenSearch => {
            handle_opensearch_request(
                State((*state.opensearch_state).clone()),
                method,
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Athena => {
            handle_athena_request(
                State((*state.athena_state).clone()),
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Rds => {
            handle_rds_request(
                State((*state.rds_state).clone()),
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::ElastiCache => {
            handle_elasticache_request(
                State((*state.elasticache_state).clone()),
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Redshift => {
            handle_redshift_request(
                State((*state.redshift_state).clone()),
                uri,
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Acm => {
            handle_acm_request(
                State((*state.acm_state).clone()),
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::WafV2 => {
            handle_wafv2_request(
                State((*state.wafv2_state).clone()),
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::Organizations => {
            handle_organizations_request(
                State((*state.organizations_state).clone()),
                headers,
                body_bytes,
            )
            .await
        }
        AwsService::DynamoDbStreams => {
            handle_dynamodb_request(state.dynamodb_engine.clone(), reconstructed_req).await
        }
        AwsService::Internal => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"status":"healthy","service":"ruststack"}"#))
            .unwrap(),
        AwsService::Unknown => handle_s3_request(state.s3_storage.clone(), reconstructed_req).await,
    }
}
