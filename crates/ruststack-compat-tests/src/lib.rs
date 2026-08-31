use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Method, Request, Response, StatusCode};
use axum::Router;
use bytes::Bytes;
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

#[derive(Clone)]
pub struct RustStackTestClient {
    pub app: Router,
    pub account_id: String,
    pub region: String,
}

impl RustStackTestClient {
    pub fn new() -> Self {
        let account_id = "000000000000".to_string();
        let region = "us-east-1".to_string();

        let s3_storage = Arc::new(InMemoryStorage::new());
        let sqs_engine = Arc::new(SqsEngine::new(account_id.clone(), region.clone()));
        let sns_engine = Arc::new(SnsEngine::new(
            sqs_engine.clone(),
            account_id.clone(),
            region.clone(),
        ));
        let eventbridge_engine = Arc::new(EventBridgeEngine::new(
            sqs_engine.clone(),
            sns_engine.clone(),
            account_id.clone(),
            region.clone(),
        ));
        let ssm_engine = Arc::new(SsmEngine::new(account_id.clone(), region.clone()));
        let secretsmanager_engine = Arc::new(SecretsManagerEngine::new(
            account_id.clone(),
            region.clone(),
        ));
        let sts_engine = Arc::new(StsEngine::new(account_id.clone(), region.clone()));
        let dynamodb_engine = Arc::new(DynamoDbEngine::new(account_id.clone(), region.clone()));
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
            chaos_engine: Arc::new(ruststack_core::ChaosEngine::new()),
            region: region.clone(),
            account_id: account_id.clone(),
        };

        let app = create_router(state);
        Self {
            app,
            account_id,
            region,
        }
    }

    pub async fn send_request(&self, req: Request<Body>) -> Response<Body> {
        self.app.clone().oneshot(req).await.unwrap()
    }

    pub async fn call_json(&self, target: &str, payload: Value) -> (StatusCode, Value) {
        let req = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("x-amz-target", target)
            .header("content-type", "application/x-amz-json-1.0")
            .body(Body::from(payload.to_string()))
            .unwrap();

        let resp = self.send_request(req).await;
        let status = resp.status();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let val: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        (status, val)
    }

    pub async fn call_query(&self, uri: &str, params: &[(&str, &str)]) -> (StatusCode, String) {
        let encoded: String = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish();

        let req = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(encoded))
            .unwrap();

        let resp = self.send_request(req).await;
        let status = resp.status();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8_lossy(&body).to_string();
        (status, text)
    }

    pub async fn call_s3(
        &self,
        method: Method,
        uri: &str,
        headers: HeaderMap,
        body: Bytes,
    ) -> (StatusCode, HeaderMap, Bytes) {
        let mut builder = Request::builder().method(method).uri(uri);
        for (k, v) in headers.iter() {
            builder = builder.header(k, v);
        }
        let req = builder.body(Body::from(body)).unwrap();

        let resp = self.send_request(req).await;
        let status = resp.status();
        let resp_headers = resp.headers().clone();
        let resp_body = resp.into_body().collect().await.unwrap().to_bytes();
        (status, resp_headers, resp_body)
    }

    pub async fn call_s3_virtual_host(
        &self,
        bucket: &str,
        method: Method,
        path: &str,
        headers: HeaderMap,
        body: Bytes,
    ) -> (StatusCode, HeaderMap, Bytes) {
        let mut req_headers = headers;
        req_headers.insert(
            "host",
            HeaderValue::from_str(&format!("{}.localhost:4566", bucket)).unwrap(),
        );
        self.call_s3(method, path, req_headers, body).await
    }
}

impl Default for RustStackTestClient {
    fn default() -> Self {
        Self::new()
    }
}
