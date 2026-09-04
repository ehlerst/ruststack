#![recursion_limit = "256"]

use clap::{Parser, Subcommand};
use ruststack_dynamodb::DynamoDbEngine;
use ruststack_eventbridge::EventBridgeEngine;
use ruststack_s3::{InMemoryStorage, S3Storage};
use ruststack_secretsmanager::SecretsManagerEngine;
use ruststack_server::state_api::RustStackStateSnapshot;
use ruststack_server::{create_router, AppState, Opts};
use ruststack_sns::SnsEngine;
use ruststack_sqs::SqsEngine;
use ruststack_ssm::SsmEngine;
use ruststack_sts::StsEngine;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

#[derive(Parser, Debug)]
#[command(
    name = "ruststack",
    about = "⚡ Blazing-fast, lightweight local AWS cloud emulator written in Rust",
    version = env!("CARGO_PKG_VERSION")
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[command(flatten)]
    pub server_opts: Opts,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the local RustStack gateway server (default)
    Start(Opts),

    /// Query the status and health of a running RustStack instance
    Status {
        #[arg(
            long,
            default_value = "http://localhost:4566",
            env = "AWS_ENDPOINT_URL"
        )]
        endpoint: String,
    },

    /// Manage cluster state, snapshots, and resets
    State {
        #[command(subcommand)]
        command: StateCommands,
    },

    /// Manage Chaos Engineering and fault injection rules
    Chaos {
        #[command(subcommand)]
        command: ChaosCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum StateCommands {
    /// Dump full cluster state to stdout or a file
    Dump {
        #[arg(
            long,
            default_value = "http://localhost:4566",
            env = "AWS_ENDPOINT_URL"
        )]
        endpoint: String,
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Load cluster state from a snapshot JSON file
    Load {
        file: String,
        #[arg(
            long,
            default_value = "http://localhost:4566",
            env = "AWS_ENDPOINT_URL"
        )]
        endpoint: String,
    },
    /// Atomically reset cluster state across all or selective services
    Reset {
        #[arg(
            long,
            default_value = "http://localhost:4566",
            env = "AWS_ENDPOINT_URL"
        )]
        endpoint: String,
        #[arg(short, long)]
        services: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ChaosCommands {
    /// List all active chaos rules
    List {
        #[arg(
            long,
            default_value = "http://localhost:4566",
            env = "AWS_ENDPOINT_URL"
        )]
        endpoint: String,
    },
    /// Add a new chaos fault injection rule
    Add {
        #[arg(
            long,
            default_value = "http://localhost:4566",
            env = "AWS_ENDPOINT_URL"
        )]
        endpoint: String,
        #[arg(short, long)]
        service: Option<String>,
        #[arg(short, long)]
        action: Option<String>,
        #[arg(short, long, default_value = "1.0")]
        probability: f64,
        #[arg(long)]
        status: Option<u16>,
        #[arg(long)]
        error_code: Option<String>,
        #[arg(long)]
        error_message: Option<String>,
        #[arg(long)]
        latency_ms: Option<u64>,
        #[arg(long)]
        latency_jitter_ms: Option<u64>,
        #[arg(long)]
        limit_times: Option<u32>,
    },
    /// Clear all chaos rules
    Reset {
        #[arg(
            long,
            default_value = "http://localhost:4566",
            env = "AWS_ENDPOINT_URL"
        )]
        endpoint: String,
    },
    /// Enable chaos engine globally
    Enable {
        #[arg(
            long,
            default_value = "http://localhost:4566",
            env = "AWS_ENDPOINT_URL"
        )]
        endpoint: String,
    },
    /// Disable chaos engine globally
    Disable {
        #[arg(
            long,
            default_value = "http://localhost:4566",
            env = "AWS_ENDPOINT_URL"
        )]
        endpoint: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => run_server(cli.server_opts).await,
        Some(Commands::Start(opts)) => run_server(opts).await,
        Some(Commands::Status { endpoint }) => run_status(&endpoint).await,
        Some(Commands::State { command }) => run_state_command(command).await,
        Some(Commands::Chaos { command }) => run_chaos_command(command).await,
    }
}

async fn run_status(endpoint: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/_ruststack/info", endpoint.trim_end_matches('/'));
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await?;
            println!("⚡ RustStack is running!");
            println!("{}", serde_json::to_string_pretty(&json)?);
            Ok(())
        }
        Ok(resp) => {
            eprintln!("RustStack returned non-200 status: {}", resp.status());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Failed to connect to RustStack at {}: {}", endpoint, e);
            std::process::exit(1);
        }
    }
}

async fn run_state_command(command: StateCommands) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    match command {
        StateCommands::Dump { endpoint, output } => {
            let url = format!("{}/_ruststack/state/dump", endpoint.trim_end_matches('/'));
            let resp = client.get(&url).send().await?;
            let body = resp.text().await?;
            if let Some(out_path) = output {
                std::fs::write(&out_path, &body)?;
                println!("✅ Snapshot successfully exported to {}", out_path);
            } else {
                println!("{}", body);
            }
        }
        StateCommands::Load { file, endpoint } => {
            let url = format!("{}/_ruststack/state/load", endpoint.trim_end_matches('/'));
            let content = std::fs::read_to_string(&file)?;
            let resp = client
                .post(&url)
                .header("content-type", "application/json")
                .body(content)
                .send()
                .await?;
            if resp.status().is_success() {
                println!("✅ State successfully restored from {}", file);
            } else {
                eprintln!("❌ Failed to restore state: {}", resp.text().await?);
                std::process::exit(1);
            }
        }
        StateCommands::Reset { endpoint, services } => {
            let url = format!("{}/_ruststack/state/reset", endpoint.trim_end_matches('/'));
            let payload = if let Some(svcs) = services {
                let list: Vec<String> = svcs.split(',').map(|s| s.trim().to_string()).collect();
                json!({ "services": list })
            } else {
                json!({})
            };
            let resp = client
                .post(&url)
                .header("content-type", "application/json")
                .body(payload.to_string())
                .send()
                .await?;
            if resp.status().is_success() {
                println!("✅ RustStack state reset successfully.");
            } else {
                eprintln!("❌ Failed to reset state: {}", resp.text().await?);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

async fn run_chaos_command(command: ChaosCommands) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    match command {
        ChaosCommands::List { endpoint } => {
            let url = format!("{}/_ruststack/chaos/rules", endpoint.trim_end_matches('/'));
            let resp = client.get(&url).send().await?;
            let json: serde_json::Value = resp.json().await?;
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        ChaosCommands::Add {
            endpoint,
            service,
            action,
            probability,
            status,
            error_code,
            error_message,
            latency_ms,
            latency_jitter_ms,
            limit_times,
        } => {
            let url = format!("{}/_ruststack/chaos/rules", endpoint.trim_end_matches('/'));
            let payload = json!({
                "service": service,
                "action": action,
                "probability": probability,
                "error_status": status,
                "error_code": error_code,
                "error_message": error_message,
                "latency_ms": latency_ms,
                "latency_jitter_ms": latency_jitter_ms,
                "limit_times": limit_times,
            });
            let resp = client
                .post(&url)
                .header("content-type", "application/json")
                .body(payload.to_string())
                .send()
                .await?;
            if resp.status().is_success() {
                println!(
                    "✅ Chaos rule registered successfully: {}",
                    resp.text().await?
                );
            } else {
                eprintln!("❌ Failed to register chaos rule: {}", resp.text().await?);
                std::process::exit(1);
            }
        }
        ChaosCommands::Reset { endpoint } => {
            let url = format!("{}/_ruststack/chaos/reset", endpoint.trim_end_matches('/'));
            client.post(&url).send().await?;
            println!("✅ All chaos rules cleared.");
        }
        ChaosCommands::Enable { endpoint } => {
            let url = format!("{}/_ruststack/chaos/enable", endpoint.trim_end_matches('/'));
            client.post(&url).send().await?;
            println!("✅ Chaos engine enabled.");
        }
        ChaosCommands::Disable { endpoint } => {
            let url = format!(
                "{}/_ruststack/chaos/disable",
                endpoint.trim_end_matches('/')
            );
            client.post(&url).send().await?;
            println!("✅ Chaos engine disabled.");
        }
    }
    Ok(())
}

async fn run_server(opts: Opts) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ruststack=info,tower_http=info".into()),
        )
        .init();

    let addr: SocketAddr = format!("{}:{}", opts.host, opts.port).parse()?;

    info!("Starting RustStack on http://{}", addr);
    info!("Active services: {}", opts.services);
    info!("Web Admin UI available at http://{}/_ruststack/ui/", addr);

    let s3_storage = Arc::new(InMemoryStorage::new());
    let sqs_engine = Arc::new(SqsEngine::new(opts.account_id.clone(), opts.region.clone()));
    let sns_engine = Arc::new(SnsEngine::new(
        sqs_engine.clone(),
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let eventbridge_engine = Arc::new(EventBridgeEngine::new(
        sqs_engine.clone(),
        sns_engine.clone(),
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let ssm_engine = Arc::new(SsmEngine::new(opts.account_id.clone(), opts.region.clone()));
    let secretsmanager_engine = Arc::new(SecretsManagerEngine::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let sts_engine = Arc::new(StsEngine::new(opts.account_id.clone(), opts.region.clone()));
    let dynamodb_engine = Arc::new(DynamoDbEngine::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let kms_state = Arc::new(ruststack_kms::KmsState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let logs_state = Arc::new(ruststack_logs::LogsState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let iam_state = Arc::new(ruststack_iam::IamState::new(opts.account_id.clone()));
    let cloudwatch_state = Arc::new(ruststack_cloudwatch::CloudWatchState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let ses_state = Arc::new(ruststack_ses::SesState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let kinesis_state = Arc::new(ruststack_kinesis::KinesisState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let lambda_state = Arc::new(ruststack_lambda::LambdaState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let cognito_state = Arc::new(ruststack_cognito::CognitoState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let apigateway_state = Arc::new(ruststack_apigateway::ApiGatewayState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let route53_state = Arc::new(ruststack_route53::Route53State::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let stepfunctions_state = Arc::new(ruststack_stepfunctions::StepFunctionsState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let cloudformation_state = Arc::new(ruststack_cloudformation::CloudFormationState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let ecr_state = Arc::new(ruststack_ecr::EcrState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let ecs_state = Arc::new(ruststack_ecs::EcsState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let ec2_state = Arc::new(ruststack_ec2::Ec2State::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let elbv2_state = Arc::new(ruststack_elbv2::Elbv2State::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let bedrock_state = Arc::new(ruststack_bedrock::BedrockState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let opensearch_state = Arc::new(ruststack_opensearch::OpenSearchState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let athena_state = Arc::new(ruststack_athena::AthenaState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let rds_state = Arc::new(ruststack_rds::RdsState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let elasticache_state = Arc::new(ruststack_elasticache::ElastiCacheState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let redshift_state = Arc::new(ruststack_redshift::RedshiftState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let acm_state = Arc::new(ruststack_acm::AcmState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let wafv2_state = Arc::new(ruststack_wafv2::Wafv2State::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let organizations_state = Arc::new(ruststack_organizations::OrganizationsState::new(
        opts.account_id.clone(),
        opts.region.clone(),
    ));
    let chaos_engine = Arc::new(ruststack_core::ChaosEngine::new());

    let state = AppState {
        s3_storage: s3_storage.clone(),
        sqs_engine: sqs_engine.clone(),
        sns_engine: sns_engine.clone(),
        eventbridge_engine: eventbridge_engine.clone(),
        ssm_engine: ssm_engine.clone(),
        secretsmanager_engine: secretsmanager_engine.clone(),
        sts_engine: sts_engine.clone(),
        dynamodb_engine: dynamodb_engine.clone(),
        kms_state: kms_state.clone(),
        logs_state: logs_state.clone(),
        iam_state: iam_state.clone(),
        cloudwatch_state: cloudwatch_state.clone(),
        ses_state: ses_state.clone(),
        kinesis_state: kinesis_state.clone(),
        lambda_state: lambda_state.clone(),
        cognito_state: cognito_state.clone(),
        apigateway_state: apigateway_state.clone(),
        route53_state: route53_state.clone(),
        stepfunctions_state: stepfunctions_state.clone(),
        cloudformation_state: cloudformation_state.clone(),
        ecr_state: ecr_state.clone(),
        ecs_state: ecs_state.clone(),
        ec2_state: ec2_state.clone(),
        elbv2_state: elbv2_state.clone(),
        bedrock_state: bedrock_state.clone(),
        opensearch_state: opensearch_state.clone(),
        athena_state: athena_state.clone(),
        rds_state: rds_state.clone(),
        elasticache_state: elasticache_state.clone(),
        redshift_state: redshift_state.clone(),
        acm_state: acm_state.clone(),
        wafv2_state: wafv2_state.clone(),
        organizations_state: organizations_state.clone(),
        chaos_engine,
        region: opts.region.clone(),
        account_id: opts.account_id.clone(),
    };

    // Auto-Disk Persistence Load on Startup
    let persist_path = if let Some(ref dir) = opts.data_dir {
        Some(format!("{}/state.json", dir.trim_end_matches('/')))
    } else if opts.persistence {
        Some("./.ruststack_data/state.json".to_string())
    } else {
        None
    };

    if let Some(ref path) = persist_path {
        if std::path::Path::new(path).exists() {
            match std::fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str::<RustStackStateSnapshot>(&content) {
                    Ok(snapshot) => {
                        s3_storage.load_state(snapshot.s3);
                        sqs_engine.load_state(snapshot.sqs);
                        sns_engine.load_state(snapshot.sns);
                        eventbridge_engine.load_state(snapshot.eventbridge);
                        ssm_engine.load_state(snapshot.ssm);
                        secretsmanager_engine.load_state(snapshot.secretsmanager);
                        sts_engine.load_state(snapshot.sts);
                        dynamodb_engine.load_state(snapshot.dynamodb);
                        kms_state.import_snapshot(snapshot.kms);
                        logs_state.import_snapshot(snapshot.logs);
                        iam_state.import_snapshot(snapshot.iam);
                        cloudwatch_state.import_snapshot(snapshot.cloudwatch);
                        ses_state.import_snapshot(snapshot.ses);
                        kinesis_state.import_snapshot(snapshot.kinesis);
                        lambda_state.import_snapshot(snapshot.lambda);
                        cognito_state.import_snapshot(snapshot.cognito);
                        apigateway_state.import_snapshot(snapshot.apigateway);
                        route53_state.import_snapshot(snapshot.route53);
                        stepfunctions_state.import_snapshot(snapshot.stepfunctions);
                        cloudformation_state.import_snapshot(snapshot.cloudformation);
                        ecr_state.import_snapshot(snapshot.ecr);
                        ecs_state.import_snapshot(snapshot.ecs);
                        ec2_state.import_snapshot(snapshot.ec2);
                        elbv2_state.import_snapshot(snapshot.elbv2);
                        bedrock_state.import_snapshot(snapshot.bedrock);
                        opensearch_state.import_snapshot(snapshot.opensearch);
                        athena_state.import_snapshot(snapshot.athena);
                        rds_state.import_snapshot(snapshot.rds);
                        elasticache_state.import_snapshot(snapshot.elasticache);
                        redshift_state.import_snapshot(snapshot.redshift);
                        acm_state.import_snapshot(snapshot.acm);
                        wafv2_state.import_snapshot(snapshot.wafv2);
                        organizations_state.import_snapshot(snapshot.organizations);
                        info!("💾 Auto-loaded persistent cluster state from {}", path);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse persistent state file {}: {}", path, e)
                    }
                },
                Err(e) => tracing::warn!("Failed to read persistent state file {}: {}", path, e),
            }
        }
    }

    let app = create_router(state.clone());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("RustStack gateway listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Auto-Disk Persistence Save on Shutdown
    if let Some(ref path) = persist_path {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let snapshot = RustStackStateSnapshot {
            version: 1,
            created_at: chrono::Utc::now(),
            region: opts.region,
            account_id: opts.account_id,
            s3: s3_storage.dump_state(),
            sqs: sqs_engine.dump_state(),
            sns: sns_engine.dump_state(),
            eventbridge: eventbridge_engine.dump_state(),
            ssm: ssm_engine.dump_state(),
            secretsmanager: secretsmanager_engine.dump_state(),
            sts: sts_engine.dump_state(),
            dynamodb: dynamodb_engine.dump_state(),
            kms: kms_state.export_snapshot(),
            logs: logs_state.export_snapshot(),
            iam: iam_state.export_snapshot(),
            cloudwatch: cloudwatch_state.export_snapshot(),
            ses: ses_state.export_snapshot(),
            kinesis: kinesis_state.export_snapshot(),
            lambda: lambda_state.export_snapshot(),
            cognito: cognito_state.export_snapshot(),
            apigateway: apigateway_state.export_snapshot(),
            route53: route53_state.export_snapshot(),
            stepfunctions: stepfunctions_state.export_snapshot(),
            cloudformation: cloudformation_state.export_snapshot(),
            ecr: ecr_state.export_snapshot(),
            ecs: ecs_state.export_snapshot(),
            ec2: ec2_state.export_snapshot(),
            elbv2: elbv2_state.export_snapshot(),
            bedrock: bedrock_state.export_snapshot(),
            opensearch: opensearch_state.export_snapshot(),
            athena: athena_state.export_snapshot(),
            rds: rds_state.export_snapshot(),
            elasticache: elasticache_state.export_snapshot(),
            redshift: redshift_state.export_snapshot(),
            acm: acm_state.export_snapshot(),
            wafv2: wafv2_state.export_snapshot(),
            organizations: organizations_state.export_snapshot(),
        };
        if let Ok(json_str) = serde_json::to_string_pretty(&snapshot) {
            if let Ok(()) = std::fs::write(path, json_str) {
                info!("💾 Auto-saved persistent cluster state to {}", path);
            }
        }
    }

    info!("RustStack shut down successfully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
