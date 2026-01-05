//! Health check endpoints for Kubernetes-style probes.
//!
//! - `/livez` - Basic liveness probe (immediate 200, no checks)
//! - `/healthz` - SSR pool stats (fast, passive stats)
//! - `/readyz` - Readiness probe (active SSR render check)

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use calendsync_ssr::{HealthStatus, SsrPoolStats};

use crate::state::AppState;

/// GET /livez - Basic liveness probe.
///
/// Returns 200 immediately. Used to check if the server is accepting connections.
/// Does NOT wait for SSR initialization.
#[axum::debug_handler]
pub async fn livez() -> StatusCode {
    StatusCode::OK
}

/// GET /healthz - SSR pool stats (passive stats, no render).
///
/// Returns pool statistics without sending a render request.
/// Fast endpoint suitable for frequent liveness checks.
#[axum::debug_handler]
pub async fn healthz(State(state): State<AppState>) -> Response {
    let Some(ssr_pool) = state.get_ssr_pool().await else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "SSR pool not initialized"
            })),
        )
            .into_response();
    };

    (StatusCode::OK, Json(ssr_pool.stats())).into_response()
}

/// GET /readyz - Readiness probe (active SSR health check).
///
/// Sends a minimal render probe to verify workers can process requests.
/// Returns 200 with health status if healthy, 503 if unhealthy.
#[axum::debug_handler]
pub async fn readyz(State(state): State<AppState>) -> Response {
    let Some(ssr_pool) = state.get_ssr_pool().await else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "healthy": false,
                "error": "SSR pool not initialized"
            })),
        )
            .into_response();
    };

    match ssr_pool.health_check().await {
        Ok(status) if status.healthy => (StatusCode::OK, Json(status)).into_response(),
        Ok(status) => (StatusCode::SERVICE_UNAVAILABLE, Json(status)).into_response(),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthStatus {
                healthy: false,
                latency_ms: 0,
                worker_idx: 0,
                stats: SsrPoolStats {
                    worker_count: 0,
                    workers_with_capacity: 0,
                },
                error: Some(e.to_string()),
            }),
        )
            .into_response(),
    }
}
