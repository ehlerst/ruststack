use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{any, get};
use axum::Router;
use clap::Parser;
use http_body_util::BodyExt;
use ruststack_core::{AwsService, Dispatcher};
use ruststack_dynamodb::{handle_dynamodb_request, DynamoDbEngine};
use ruststack_eventbridge::{handle_eventbridge_request, EventBridgeEngine};
use ruststack_s3::{handle_s3_request, S3NotificationTarget, S3Storage};
use ruststack_secretsmanager::{handle_secretsmanager_request, SecretsManagerEngine};
use ruststack_sns::{handle_sns_request, SnsEngine};
use ruststack_sqs::{handle_sqs_request, SqsEngine};
use ruststack_ssm::{handle_ssm_request, SsmEngine};
use ruststack_sts::{handle_sts_request, StsEngine};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

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
        default_value = "s3,sqs,sns,events,ssm,secretsmanager,sts,dynamodb",
        env = "SERVICES"
    )]
    pub services: String,

    #[arg(long, default_value = "us-east-1", env = "DEFAULT_REGION")]
    pub region: String,

    #[arg(long, default_value = "000000000000", env = "ACCOUNT_ID")]
    pub account_id: String,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            port: 4566,
            host: "0.0.0.0".to_string(),
            services: "s3,sqs,sns,events,ssm,secretsmanager,sts,dynamodb".to_string(),
            region: "us-east-1".to_string(),
            account_id: "000000000000".to_string(),
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

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any);

    Router::new()
        .route("/_ruststack/health", get(health_check))
        .route("/_ruststack/info", get(info_handler))
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
        "services": ["s3", "sqs", "sns", "events", "ssm", "secretsmanager", "sts", "dynamodb"],
        "region": state.region,
        "account_id": state.account_id,
        "features": {
            "s3": ["buckets", "objects", "multipart", "byte-range", "virtual-hosting", "bucket-notifications"],
            "sqs": ["standard-queue", "fifo-queue", "dlq-redrive", "long-polling", "batching", "visibility-timeout", "query-and-json-protocols"],
            "sns": ["topics", "subscriptions", "sqs-fanout", "filter-policy", "raw-message-delivery", "query-and-json-protocols"],
            "events": ["event-buses", "rules", "pattern-matching", "targets-sqs-sns", "json-protocol"],
            "ssm": ["parameters", "hierarchical-paths", "versioning", "secure-string", "json-protocol"],
            "secretsmanager": ["secrets", "version-stages", "rotation", "binary-and-string", "json-protocol"],
            "sts": ["caller-identity", "assume-role", "session-tokens", "query-and-json-protocols"],
            "dynamodb": ["tables", "crud", "query", "scan", "key-conditions", "filter-expressions", "gsi-lsi", "batching", "json-protocol"]
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

    // Check service classification with body peek for Form Query actions
    let service = Dispatcher::classify_request(&method, &uri, &headers, Some(&body_bytes));
    let reconstructed_req = Request::from_parts(parts, Body::from(body_bytes));

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
        AwsService::Internal => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"status":"healthy","service":"ruststack"}"#))
            .unwrap(),
        AwsService::Unknown => handle_s3_request(state.s3_storage.clone(), reconstructed_req).await,
    }
}
