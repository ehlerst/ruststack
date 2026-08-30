use clap::{Parser, Subcommand};
use ruststack_dynamodb::DynamoDbEngine;
use ruststack_eventbridge::EventBridgeEngine;
use ruststack_s3::InMemoryStorage;
use ruststack_secretsmanager::SecretsManagerEngine;
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
    let chaos_engine = Arc::new(ruststack_core::ChaosEngine::new());

    let state = AppState {
        s3_storage,
        sqs_engine,
        sns_engine,
        eventbridge_engine,
        ssm_engine,
        secretsmanager_engine,
        sts_engine,
        dynamodb_engine,
        chaos_engine,
        region: opts.region,
        account_id: opts.account_id,
    };

    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("RustStack gateway listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

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
