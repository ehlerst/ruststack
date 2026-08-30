use clap::Parser;
use ruststack_eventbridge::EventBridgeEngine;
use ruststack_s3::InMemoryStorage;
use ruststack_server::{create_router, AppState, Opts};
use ruststack_sns::SnsEngine;
use ruststack_sqs::SqsEngine;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ruststack=info,tower_http=info".into()),
        )
        .init();

    let opts = Opts::parse();
    let addr: SocketAddr = format!("{}:{}", opts.host, opts.port).parse()?;

    info!("Starting RustStack on http://{}", addr);
    info!("Active services: {}", opts.services);

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

    let state = AppState {
        s3_storage,
        sqs_engine,
        sns_engine,
        eventbridge_engine,
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
