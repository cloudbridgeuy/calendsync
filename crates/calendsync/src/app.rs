use std::time::Duration;

use axum::{
    http::{header, Method, StatusCode},
    routing::{get, patch},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::{
    handlers::{
        calendar_react::{calendar_react_ssr, calendar_react_ssr_entry},
        entries::{
            create_entry, delete_entry, get_entry, list_calendar_entries, list_entries,
            toggle_entry, update_entry,
        },
        events::events_sse,
        health::{healthz, readyz},
        static_files::serve_static,
    },
    state::AppState,
};

/// Create the application router with all routes and middleware.
pub fn create_app(state: AppState) -> Router {
    // CORS configuration for API endpoints
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::CONTENT_TYPE]);

    // API routes with CORS
    let api_routes = Router::new()
        // Entry routes
        .route("/entries", get(list_entries).post(create_entry))
        .route("/entries/calendar", get(list_calendar_entries))
        .route(
            "/entries/{id}",
            get(get_entry).put(update_entry).delete(delete_entry),
        )
        .route("/entries/{id}/toggle", patch(toggle_entry))
        // SSE events stream for real-time updates
        .route("/events", get(events_sse))
        .layer(cors);

    // Main application router
    Router::new()
        .route("/calendar/{calendar_id}", get(calendar_react_ssr))
        .route(
            "/calendar/{calendar_id}/entry",
            get(calendar_react_ssr_entry),
        )
        .route("/dist/{*filename}", get(serve_static))
        // Health check routes (Kubernetes-style)
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .nest("/api", api_routes)
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(10),
        ))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_healthz_without_ssr_pool() {
        let state = AppState::default();
        let app = create_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 503 when SSR pool is not initialized
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_readyz_without_ssr_pool() {
        let state = AppState::default();
        let app = create_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 503 when SSR pool is not initialized
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_list_entries_empty() {
        let state = AppState::default();
        let app = create_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/entries")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

        assert!(json.is_empty());
    }

    #[tokio::test]
    async fn test_get_nonexistent_entry() {
        let state = AppState::default();
        let app = create_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/entries/00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
