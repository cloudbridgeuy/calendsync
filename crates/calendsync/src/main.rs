mod app;
mod handlers;
mod mock_data;
mod models;
mod state;
mod storage;

use std::path::Path;

use anyhow::Result;
use calendsync_ssr::{SsrPool, SsrPoolConfig};
use clap::Parser;
use listenfd::ListenFd;
use tokio::{net::TcpListener, signal};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{app::create_app, state::AppState};

/// CalendSync - Create calendars to sync with your friends
#[derive(Parser, Debug)]
#[command(name = "calendsync")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Host address to bind the server to
    #[arg(long, short = 'H', default_value = "0.0.0.0", env = "HOST")]
    host: String,

    /// Port to listen on
    #[arg(long, short, default_value = "3000", env = "PORT")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing subscriber
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "calendsync=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize SSR pool
    let ssr_pool = init_ssr_pool()?;

    // Create application state with demo data and SSR pool
    let state = AppState::with_demo_data().with_ssr_pool(ssr_pool);

    // Build the application router
    let app = create_app(state.clone());

    // Auto-reload support via listenfd
    let mut listenfd = ListenFd::from_env();
    let listener = match listenfd.take_tcp_listener(0)? {
        // If we are given a tcp listener on listen fd 0, use that one
        Some(listener) => {
            listener.set_nonblocking(true)?;
            TcpListener::from_std(listener)?
        }
        // Otherwise fall back to CLI-specified host:port
        None => {
            let addr = format!("{}:{}", cli.host, cli.port);
            TcpListener::bind(&addr).await?
        }
    };

    tracing::info!("listening on {}", listener.local_addr()?);

    // Run the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(state))
        .await?;

    tracing::info!("Server stopped");
    Ok(())
}

/// Initialize the SSR worker pool.
///
/// Reads the server bundle path from the manifest and creates a pool
/// with workers based on available parallelism.
///
/// In dev mode (DEV_MODE env var set), reads manifest from disk to support
/// hot-reload. In production, uses compiled-in manifest.
fn init_ssr_pool() -> Result<SsrPool> {
    let bundle_path = if std::env::var("DEV_MODE").is_ok() {
        // Dev mode: read manifest from disk (hot-reloadable)
        resolve_bundle_path_runtime()?
    } else {
        // Production: use compiled-in manifest
        resolve_bundle_path_compiled()
    };

    // Determine worker count based on available parallelism
    let worker_count = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4);

    // Create pool config (10s timeout, production)
    let pool_config = SsrPoolConfig::with_defaults(worker_count)?;

    tracing::info!(
        workers = worker_count,
        bundle = %bundle_path.display(),
        dev_mode = std::env::var("DEV_MODE").is_ok(),
        "Initializing SSR pool"
    );

    let pool = SsrPool::new(pool_config, &bundle_path)?;

    Ok(pool)
}

/// Resolve bundle path from disk manifest (dev mode).
fn resolve_bundle_path_runtime() -> Result<std::path::PathBuf> {
    let frontend_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend");
    let manifest_path = frontend_dir.join("manifest.json");

    let manifest_str = std::fs::read_to_string(&manifest_path)?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_str)?;

    let server_bundle_name = manifest
        .get("calendsync.js")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync-server.js");

    Ok(frontend_dir.join("dist").join(server_bundle_name))
}

/// Resolve bundle path from compiled-in manifest (production).
fn resolve_bundle_path_compiled() -> std::path::PathBuf {
    let manifest_str = include_str!("../../frontend/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(manifest_str).unwrap_or(serde_json::json!({}));

    let server_bundle_name = manifest
        .get("calendsync.js")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync-server.js");

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../frontend/dist")
        .join(server_bundle_name)
}

/// Wait for shutdown signals (Ctrl+C or SIGTERM) and notify SSE handlers.
async fn shutdown_signal(state: AppState) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, shutting down...");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, shutting down...");
        }
    }

    // Signal SSE handlers to close their connections
    state.signal_shutdown();
}
