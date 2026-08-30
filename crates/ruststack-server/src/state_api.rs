use crate::app::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use chrono::{DateTime, Utc};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustStackStateSnapshot {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub region: String,
    pub account_id: String,
    pub s3: ruststack_s3::S3Snapshot,
    pub sqs: ruststack_sqs::SqsSnapshot,
    pub sns: ruststack_sns::SnsSnapshot,
    pub eventbridge: ruststack_eventbridge::EventBridgeSnapshot,
    pub ssm: ruststack_ssm::SsmSnapshot,
    pub secretsmanager: ruststack_secretsmanager::SecretsManagerSnapshot,
    pub sts: ruststack_sts::StsSnapshot,
    pub dynamodb: ruststack_dynamodb::DynamoDbSnapshot,
    pub kms: ruststack_kms::KmsStateSnapshot,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResetStateRequest {
    pub services: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DumpStateRequest {
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoadStateRequest {
    pub file_path: Option<String>,
    pub snapshot: Option<RustStackStateSnapshot>,
}

pub async fn state_reset_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Response<Body> {
    let body_bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(e.to_string()))
                .unwrap();
        }
    };

    let reset_req: ResetStateRequest = if !body_bytes.is_empty() {
        serde_json::from_slice(&body_bytes).unwrap_or_default()
    } else {
        ResetStateRequest::default()
    };

    let all_services = vec![
        "s3",
        "sqs",
        "sns",
        "eventbridge",
        "ssm",
        "secretsmanager",
        "sts",
        "dynamodb",
        "kms",
    ];

    let target_services = match reset_req.services {
        Some(ref list) if !list.is_empty() => list.clone(),
        _ => all_services.into_iter().map(String::from).collect(),
    };

    for svc in &target_services {
        match svc.to_lowercase().as_str() {
            "s3" => state.s3_storage.reset(),
            "sqs" => state.sqs_engine.reset(),
            "sns" => state.sns_engine.reset(),
            "eventbridge" | "events" => state.eventbridge_engine.reset(),
            "ssm" => state.ssm_engine.reset(),
            "secretsmanager" => state.secretsmanager_engine.reset(),
            "sts" => state.sts_engine.reset(),
            "dynamodb" => state.dynamodb_engine.reset(),
            "kms" => state.kms_state.reset(),
            _ => {}
        }
    }

    let resp_json = json!({
        "status": "ok",
        "message": "State reset successfully",
        "reset_services": target_services,
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(resp_json.to_string()))
        .unwrap()
}

pub async fn state_dump_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Response<Body> {
    let body_bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(_) => bytes::Bytes::new(),
    };

    let dump_req: DumpStateRequest = if !body_bytes.is_empty() {
        serde_json::from_slice(&body_bytes).unwrap_or_default()
    } else {
        DumpStateRequest::default()
    };

    let snapshot = RustStackStateSnapshot {
        version: 1,
        created_at: Utc::now(),
        region: state.region.clone(),
        account_id: state.account_id.clone(),
        s3: state.s3_storage.dump_state(),
        sqs: state.sqs_engine.dump_state(),
        sns: state.sns_engine.dump_state(),
        eventbridge: state.eventbridge_engine.dump_state(),
        ssm: state.ssm_engine.dump_state(),
        secretsmanager: state.secretsmanager_engine.dump_state(),
        sts: state.sts_engine.dump_state(),
        dynamodb: state.dynamodb_engine.dump_state(),
        kms: state.kms_state.export_snapshot(),
    };

    if let Some(file_path) = dump_req.file_path {
        let json_str = match serde_json::to_string_pretty(&snapshot) {
            Ok(s) => s,
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(e.to_string()))
                    .unwrap();
            }
        };

        if let Err(e) = std::fs::write(&file_path, json_str) {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!(
                    "Failed to write snapshot to {}: {}",
                    file_path, e
                )))
                .unwrap();
        }

        let resp_json = json!({
            "status": "ok",
            "message": "Snapshot saved to file",
            "file_path": file_path
        });

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(resp_json.to_string()))
            .unwrap()
    } else {
        let json_str = serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".to_string());
        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(json_str))
            .unwrap()
    }
}

pub async fn state_load_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Response<Body> {
    let body_bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(e.to_string()))
                .unwrap();
        }
    };

    // Try parsing as direct RustStackStateSnapshot first, then as LoadStateRequest wrapper
    let snapshot: RustStackStateSnapshot =
        if let Ok(snap) = serde_json::from_slice::<RustStackStateSnapshot>(&body_bytes) {
            snap
        } else if let Ok(load_req) = serde_json::from_slice::<LoadStateRequest>(&body_bytes) {
            if let Some(snap) = load_req.snapshot {
                snap
            } else if let Some(file_path) = load_req.file_path {
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => match serde_json::from_str(&content) {
                        Ok(snap) => snap,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Invalid snapshot file format: {}", e)))
                                .unwrap();
                        }
                    },
                    Err(e) => {
                        return Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from(format!(
                                "Failed to read file {}: {}",
                                file_path, e
                            )))
                            .unwrap();
                    }
                }
            } else {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Missing snapshot or file_path in request"))
                    .unwrap();
            }
        } else {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Invalid JSON snapshot payload"))
                .unwrap();
        };

    // Restore state to all 8 services
    state.s3_storage.load_state(snapshot.s3);
    state.sqs_engine.load_state(snapshot.sqs);
    state.sns_engine.load_state(snapshot.sns);
    state.eventbridge_engine.load_state(snapshot.eventbridge);
    state.ssm_engine.load_state(snapshot.ssm);
    state
        .secretsmanager_engine
        .load_state(snapshot.secretsmanager);
    state.sts_engine.load_state(snapshot.sts);
    state.dynamodb_engine.load_state(snapshot.dynamodb);
    state.kms_state.import_snapshot(snapshot.kms);

    let resp_json = json!({
        "status": "ok",
        "message": "State loaded successfully",
        "snapshot_version": snapshot.version,
        "created_at": snapshot.created_at
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(resp_json.to_string()))
        .unwrap()
}
