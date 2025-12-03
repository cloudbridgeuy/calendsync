//! Health check endpoints for SSR pool monitoring.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use calendsync_ssr::{HealthStatus, SsrPoolStats};

use crate::state::AppState;

/// GET /health/ssr - Active health check for SSR pool.
///
/// Sends a minimal render probe to verify workers can process requests.
/// Returns 200 with health status if healthy, 503 if unhealthy.
#[axum::debug_handler]
pub async fn ssr_health(State(state): State<AppState>) -> Response {
    let Some(ssr_pool) = &state.ssr_pool else {
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

/// GET /health/ssr/stats - Passive stats (no render probe).
///
/// Returns pool statistics without sending a render request.
/// Fast endpoint for basic monitoring.
#[axum::debug_handler]
pub async fn ssr_stats(State(state): State<AppState>) -> Response {
    let Some(ssr_pool) = &state.ssr_pool else {
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
