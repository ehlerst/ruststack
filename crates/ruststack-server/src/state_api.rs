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
    pub logs: ruststack_logs::LogsStateSnapshot,
    pub iam: ruststack_iam::IamStateSnapshot,
    pub cloudwatch: ruststack_cloudwatch::CloudWatchSnapshot,
    pub ses: ruststack_ses::SesStateSnapshot,
    pub kinesis: ruststack_kinesis::KinesisStateSnapshot,
    pub lambda: ruststack_lambda::LambdaStateSnapshot,
    pub cognito: ruststack_cognito::CognitoStateSnapshot,
    pub apigateway: ruststack_apigateway::ApiGatewayStateSnapshot,
    pub route53: ruststack_route53::Route53StateSnapshot,
    pub stepfunctions: ruststack_stepfunctions::StepFunctionsStateSnapshot,
    pub cloudformation: ruststack_cloudformation::CloudFormationStateSnapshot,
    pub ecr: ruststack_ecr::EcrStateSnapshot,
    pub ecs: ruststack_ecs::EcsStateSnapshot,
    pub ec2: ruststack_ec2::Ec2StateSnapshot,
    pub elbv2: ruststack_elbv2::Elbv2StateSnapshot,
    pub bedrock: ruststack_bedrock::BedrockStateSnapshot,
    pub opensearch: ruststack_opensearch::OpenSearchStateSnapshot,
    pub athena: ruststack_athena::AthenaStateSnapshot,
    pub rds: ruststack_rds::RdsStateSnapshot,
    pub elasticache: ruststack_elasticache::ElastiCacheStateSnapshot,
    pub redshift: ruststack_redshift::RedshiftStateSnapshot,
    pub acm: ruststack_acm::AcmStateSnapshot,
    pub wafv2: ruststack_wafv2::Wafv2StateSnapshot,
    pub organizations: ruststack_organizations::OrganizationsStateSnapshot,
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
        "logs",
        "iam",
        "cloudwatch",
        "ses",
        "kinesis",
        "lambda",
        "cognito",
        "apigateway",
        "route53",
        "stepfunctions",
        "cloudformation",
        "ecr",
        "ecs",
        "ec2",
        "elbv2",
        "bedrock",
        "opensearch",
        "athena",
        "rds",
        "elasticache",
        "redshift",
        "acm",
        "wafv2",
        "organizations",
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
            "logs" => state.logs_state.reset(),
            "iam" => state.iam_state.reset(),
            "cloudwatch" => state.cloudwatch_state.reset(),
            "ses" => state.ses_state.reset(),
            "kinesis" => state.kinesis_state.reset(),
            "lambda" => state.lambda_state.reset(),
            "cognito" | "cognito-idp" => state.cognito_state.reset(),
            "apigateway" => state.apigateway_state.reset(),
            "route53" => state.route53_state.reset(),
            "stepfunctions" | "states" => state.stepfunctions_state.reset(),
            "cloudformation" => state.cloudformation_state.reset(),
            "ecr" => state.ecr_state.reset(),
            "ecs" => state.ecs_state.reset(),
            "ec2" => state.ec2_state.reset(),
            "elbv2" | "elasticloadbalancing" => state.elbv2_state.reset(),
            "bedrock" => state.bedrock_state.reset(),
            "opensearch" | "es" => state.opensearch_state.reset(),
            "athena" => state.athena_state.reset(),
            "rds" => state.rds_state.reset(),
            "elasticache" => state.elasticache_state.reset(),
            "redshift" => state.redshift_state.reset(),
            "acm" => state.acm_state.reset(),
            "wafv2" | "waf" => state.wafv2_state.reset(),
            "organizations" => state.organizations_state.reset(),
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
        logs: state.logs_state.export_snapshot(),
        iam: state.iam_state.export_snapshot(),
        cloudwatch: state.cloudwatch_state.export_snapshot(),
        ses: state.ses_state.export_snapshot(),
        kinesis: state.kinesis_state.export_snapshot(),
        lambda: state.lambda_state.export_snapshot(),
        cognito: state.cognito_state.export_snapshot(),
        apigateway: state.apigateway_state.export_snapshot(),
        route53: state.route53_state.export_snapshot(),
        stepfunctions: state.stepfunctions_state.export_snapshot(),
        cloudformation: state.cloudformation_state.export_snapshot(),
        ecr: state.ecr_state.export_snapshot(),
        ecs: state.ecs_state.export_snapshot(),
        ec2: state.ec2_state.export_snapshot(),
        elbv2: state.elbv2_state.export_snapshot(),
        bedrock: state.bedrock_state.export_snapshot(),
        opensearch: state.opensearch_state.export_snapshot(),
        athena: state.athena_state.export_snapshot(),
        rds: state.rds_state.export_snapshot(),
        elasticache: state.elasticache_state.export_snapshot(),
        redshift: state.redshift_state.export_snapshot(),
        acm: state.acm_state.export_snapshot(),
        wafv2: state.wafv2_state.export_snapshot(),
        organizations: state.organizations_state.export_snapshot(),
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

    // Restore state to all services
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
    state.logs_state.import_snapshot(snapshot.logs);
    state.iam_state.import_snapshot(snapshot.iam);
    state.cloudwatch_state.import_snapshot(snapshot.cloudwatch);
    state.ses_state.import_snapshot(snapshot.ses);
    state.kinesis_state.import_snapshot(snapshot.kinesis);
    state.lambda_state.import_snapshot(snapshot.lambda);
    state.cognito_state.import_snapshot(snapshot.cognito);
    state.apigateway_state.import_snapshot(snapshot.apigateway);
    state.route53_state.import_snapshot(snapshot.route53);
    state
        .stepfunctions_state
        .import_snapshot(snapshot.stepfunctions);
    state
        .cloudformation_state
        .import_snapshot(snapshot.cloudformation);
    state.ecr_state.import_snapshot(snapshot.ecr);
    state.ecs_state.import_snapshot(snapshot.ecs);
    state.ec2_state.import_snapshot(snapshot.ec2);
    state.elbv2_state.import_snapshot(snapshot.elbv2);
    state.bedrock_state.import_snapshot(snapshot.bedrock);
    state.opensearch_state.import_snapshot(snapshot.opensearch);
    state.athena_state.import_snapshot(snapshot.athena);
    state.rds_state.import_snapshot(snapshot.rds);
    state
        .elasticache_state
        .import_snapshot(snapshot.elasticache);
    state.redshift_state.import_snapshot(snapshot.redshift);
    state.acm_state.import_snapshot(snapshot.acm);
    state.wafv2_state.import_snapshot(snapshot.wafv2);
    state
        .organizations_state
        .import_snapshot(snapshot.organizations);

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
