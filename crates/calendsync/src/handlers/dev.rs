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

/// Request body for build error endpoint.
#[derive(serde::Deserialize)]
pub struct BuildErrorRequest {
    pub error: String,
}

/// Request body for reload endpoint with optional manifest comparison.
#[derive(serde::Deserialize, Default)]
pub struct ReloadRequest {
    /// Previous CSS filename for change detection.
    #[serde(default)]
    pub prev_css: Option<String>,
    /// Previous server JS filename for change detection.
    #[serde(default)]
    pub prev_server_js: Option<String>,
    /// Previous client JS filename for change detection.
    #[serde(default)]
    pub prev_client_js: Option<String>,
}

/// POST /_dev/reload - Reload SSR bundle (dev mode only).
///
/// Reads the new manifest, detects what changed, and takes minimal action:
/// - "none": No changes, skip reload entirely
/// - "css_only": Only CSS changed, hot-swap without pool swap or page reload
/// - "client_only": Only client JS changed, page reload without pool swap
/// - "full": Server JS changed, swap pool and reload page
///
/// This endpoint is only available when DEV_MODE environment variable is set.
#[axum::debug_handler]
pub async fn reload_ssr(
    State(state): State<AppState>,
    Json(body): Json<ReloadRequest>,
) -> impl IntoResponse {
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

    // Get all bundle names from manifest
    let new_server_js = manifest
        .get("calendsync.js")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync-server.js");

    let new_client_js = manifest
        .get("calendsync-client.js")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync-client.js");

    let new_css = manifest
        .get("calendsync.css")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync.css");

    // Handle first request (no prev values) - log and treat as full reload
    let is_first_request =
        body.prev_css.is_none() && body.prev_server_js.is_none() && body.prev_client_js.is_none();

    if is_first_request {
        tracing::info!("First reload request - no previous state, performing full reload");
    }

    // Determine what changed
    let css_changed = body.prev_css.as_ref().is_some_and(|prev| prev != new_css);
    let server_js_changed = body
        .prev_server_js
        .as_ref()
        .is_some_and(|prev| prev != new_server_js);
    let client_js_changed = body
        .prev_client_js
        .as_ref()
        .is_some_and(|prev| prev != new_client_js);

    // Determine action based on change matrix
    let change_type = if is_first_request {
        "full"
    } else {
        match (server_js_changed, client_js_changed, css_changed) {
            (false, false, false) => "none",
            (false, false, true) => "css_only",
            (false, true, _) => "client_only",
            (true, _, _) => "full",
        }
    };

    let bundle_path = frontend_dir.join("dist").join(new_server_js);

    // Only swap pool if server JS changed (or first request)
    if change_type == "full" {
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
    }

    // Signal browsers based on change type
    match change_type {
        "none" => {
            tracing::info!("No changes detected, skipping reload");
        }
        "css_only" => {
            state.signal_dev_css_reload(new_css.to_string());
            tracing::info!(css = new_css, "CSS hot-swapped (no pool swap)");
        }
        "client_only" => {
            state.signal_dev_reload();
            tracing::info!(
                client_js = new_client_js,
                "Client JS changed (no pool swap)"
            );
        }
        _ => {
            // "full"
            state.signal_dev_reload();
            tracing::info!(
                server_js = new_server_js,
                "Server JS changed, SSR pool swapped"
            );
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "bundle": bundle_path.display().to_string(),
            "css": new_css,
            "server_js": new_server_js,
            "client_js": new_client_js,
            "change_type": change_type
        })),
    )
}

/// POST /_dev/error - Report build error (dev mode only).
///
/// Called by xtask when the frontend build fails. Broadcasts the error
/// to all connected browsers for display in an error overlay.
#[axum::debug_handler]
pub async fn report_build_error(
    State(state): State<AppState>,
    Json(body): Json<BuildErrorRequest>,
) -> impl IntoResponse {
    state.signal_dev_error(body.error);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true
        })),
    )
}

/// GET /_dev/events - SSE endpoint for dev reload signals.
///
/// Browsers connect to this endpoint to receive reload and build error notifications.
/// - "reload" event: Signals browser to refresh the page
/// - "css-reload" event: Signals browser to hot-swap CSS without full reload
/// - "build-error" event: Signals browser to display error overlay
pub async fn dev_events_sse(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let mut reload_rx = state.subscribe_dev_reload();
    let mut error_rx = state.subscribe_dev_error();
    let mut css_reload_rx = state.subscribe_dev_css_reload();
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
                // CSS reload signal received
                Ok(css) = css_reload_rx.recv() => {
                    let data = serde_json::json!({ "filename": css.filename });
                    yield Ok(Event::default().event("css-reload").data(data.to_string()));
                }
                // Build error received
                Ok(error) = error_rx.recv() => {
                    let data = serde_json::json!({ "error": error.error });
                    yield Ok(Event::default().event("build-error").data(data.to_string()));
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
