use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::info;

use flowfull_rust_starter::{
    AppState, auth::BridgeValidator, build_app, cache::HybridCache, config, db,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = config::Settings::load()?;
    config::init_tracing(&settings);

    info!(
        environment = %settings.environment,
        port = settings.port,
        "starting flowfull-rust-starter"
    );

    let db = db::connect(&settings).await?;
    let cache = HybridCache::new(&settings).await?;
    let bridge_validator = BridgeValidator::new(&settings)?;
    let state = Arc::new(AppState {
        settings: settings.clone(),
        db,
        cache,
        bridge_validator,
    });

    let app = build_app(state.clone())?;
    let listener = TcpListener::bind(settings.addr()?).await?;
    info!(address = %listener.local_addr()?, "server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

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
