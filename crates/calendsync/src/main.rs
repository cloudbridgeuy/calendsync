mod app;
mod assets;
mod error;
mod handlers;
mod mock_data;
mod models;
mod state;

use anyhow::Result;
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

    // Create application state with demo data
    let state = AppState::with_demo_data();

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
