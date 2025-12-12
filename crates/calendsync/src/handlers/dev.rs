//! Development-only handlers for hot-reload.
//!
//! Only available when DEV_MODE environment variable is set.

use std::convert::Infallible;
use std::path::Path;

use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};

use calendsync_ssr::{SsrPool, SsrPoolConfig};

use crate::state::AppState;

/// POST /_dev/reload - Reload SSR bundle (dev mode only).
///
/// Reads the new manifest, loads the new bundle, and swaps the SSR pool.
/// This endpoint is only available when DEV_MODE environment variable is set.
#[axum::debug_handler]
pub async fn reload_ssr(State(state): State<AppState>) -> impl IntoResponse {
    let frontend_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend");
    let manifest_path = frontend_dir.join("manifest.json");

    // Read manifest from disk
    let manifest_str = match std::fs::read_to_string(&manifest_path) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to read manifest: {}", e)
                })),
            );
        }
    };

    // Parse manifest
    let manifest: serde_json::Value = match serde_json::from_str(&manifest_str) {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to parse manifest: {}", e)
                })),
            );
        }
    };

    // Get server bundle name from manifest
    let server_bundle_name = manifest
        .get("calendsync.js")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync-server.js");

    let bundle_path = frontend_dir.join("dist").join(server_bundle_name);

    // Create pool config
    let worker_count = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4);

    let pool_config = match SsrPoolConfig::with_defaults(worker_count) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Invalid pool config: {}", e)
                })),
            );
        }
    };

    // Create new pool
    let new_pool = match SsrPool::new(pool_config, &bundle_path) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to create SSR pool: {}", e)
                })),
            );
        }
    };

    // Swap the pool
    state.swap_ssr_pool(new_pool).await;

    // Signal browsers to reload
    state.signal_dev_reload();

    tracing::info!(bundle = %bundle_path.display(), "SSR pool reloaded");

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "bundle": bundle_path.display().to_string()
        })),
    )
}

/// GET /_dev/events - SSE endpoint for dev reload signals.
///
/// Browsers connect to this endpoint to receive reload notifications.
/// When the SSR pool is swapped, a "reload" event is sent to all connected clients.
pub async fn dev_events_sse(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let mut reload_rx = state.subscribe_dev_reload();
    let mut shutdown_rx = state.subscribe_shutdown();

    let stream = async_stream::stream! {
        // Send initial connection event
        yield Ok(Event::default().event("connected").data("{}"));

        loop {
            tokio::select! {
                // Reload signal received
                Ok(()) = reload_rx.recv() => {
                    yield Ok(Event::default().event("reload").data("{}"));
                }
                // Shutdown signal - close connection
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
