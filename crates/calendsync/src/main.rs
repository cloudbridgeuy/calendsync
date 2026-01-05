mod app;
mod cache;
mod config;
mod handlers;
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

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use std::sync::Arc;

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use calendsync_auth::{AuthConfig, AuthState};

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use calendsync_core::auth::SessionRepository;

use crate::{app::create_app, config::Config, state::AppState};

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

    // Load configuration from environment
    let config = Config::from_env();

    // Create application state with SSR pool
    let state = AppState::new(&config).await?.with_ssr_pool(ssr_pool);

    // Spawn Mock IdP server when auth-mock feature is enabled
    #[cfg(feature = "auth-mock")]
    {
        use calendsync_auth::mock_idp::MockIdpServer;

        let mock_server = MockIdpServer::new(3001);
        tokio::spawn(async move {
            if let Err(e) = mock_server.run().await {
                tracing::error!(error = %e, "Mock IdP server failed");
            }
        });
        tracing::info!("Mock IdP server started on port 3001");
    }

    // Initialize auth if configured
    #[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
    let state = {
        match create_session_store(&config).await {
            Ok(session_store) => {
                if let Some(auth_state) = setup_auth(&state, session_store).await {
                    state.with_auth(auth_state)
                } else {
                    state
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to create session store, running without auth");
                state
            }
        }
    };

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

// ============================================================================
// Auth setup (feature-gated)
// ============================================================================

/// Creates a SQLite-backed session store.
#[cfg(feature = "auth-sqlite")]
async fn create_session_store(config: &Config) -> anyhow::Result<Arc<dyn SessionRepository>> {
    use calendsync_auth::SessionStore;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::path::Path;

    // Ensure parent directory exists
    if let Some(parent) = Path::new(&config.auth_sqlite_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    // Create SQLite connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{}?mode=rwc", config.auth_sqlite_path))
        .await?;

    let store = SessionStore::new(pool);
    store.migrate().await?;

    tracing::info!(path = %config.auth_sqlite_path, "SQLite session store initialized");
    Ok(Arc::new(store))
}

/// Creates a Redis-backed session store.
#[cfg(all(feature = "auth-redis", not(feature = "auth-sqlite")))]
async fn create_session_store(config: &Config) -> anyhow::Result<Arc<dyn SessionRepository>> {
    use calendsync_auth::SessionStore;
    use fred::prelude::*;
    use std::time::Duration;

    // Create Redis pool configuration
    let redis_config = Config::from_url(&config.redis_url)?;
    let pool = Builder::from_config(redis_config)
        .build_pool(5)
        .expect("Failed to create Redis pool");

    pool.init().await?;

    // Default session TTL of 24 hours
    let session_ttl = Duration::from_secs(24 * 60 * 60);
    let store = SessionStore::new(pool, session_ttl);

    tracing::info!(url = %config.redis_url, "Redis session store initialized");
    Ok(Arc::new(store))
}

/// Creates an in-memory session store for testing/development.
#[cfg(all(
    feature = "auth-mock",
    not(feature = "auth-sqlite"),
    not(feature = "auth-redis")
))]
async fn create_session_store(_config: &Config) -> anyhow::Result<Arc<dyn SessionRepository>> {
    use crate::storage::inmemory::InMemorySessionStore;

    tracing::info!("In-memory session store initialized (auth-mock mode)");
    Ok(Arc::new(InMemorySessionStore::new()))
}

/// Sets up authentication if configured via environment variables.
///
/// Returns `None` if auth environment variables are not set (running without auth).
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
async fn setup_auth(
    state: &AppState,
    session_store: Arc<dyn SessionRepository>,
) -> Option<AuthState> {
    let config = match AuthConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::info!(error = %e, "Auth not configured, skipping");
            return None;
        }
    };

    match AuthState::new(
        session_store,
        state.user_repo.clone(),
        state.calendar_repo.clone(),
        state.membership_repo.clone(),
        config,
    )
    .await
    {
        Ok(auth_state) => {
            tracing::info!("Auth initialized successfully");
            Some(auth_state)
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to initialize auth");
            None
        }
    }
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
