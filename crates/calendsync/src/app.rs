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
        api::{create_user, delete_user, get_user, list_users},
        calendar::calendar_demo,
        calendars::{
            create_calendar, delete_calendar, get_calendar, list_calendars, update_calendar,
        },
        entries::{
            create_entry, delete_entry, get_entry, list_entries, toggle_entry, update_entry,
        },
        pages::index,
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
        // User routes
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", get(get_user).delete(delete_user))
        // Calendar routes
        .route("/calendars", get(list_calendars).post(create_calendar))
        .route(
            "/calendars/{id}",
            get(get_calendar)
                .put(update_calendar)
                .delete(delete_calendar),
        )
        // Entry routes
        .route("/entries", get(list_entries).post(create_entry))
        .route(
            "/entries/{id}",
            get(get_entry).put(update_entry).delete(delete_entry),
        )
        .route("/entries/{id}/toggle", patch(toggle_entry))
        .layer(cors);

    // Main application router
    Router::new()
        .route("/", get(index))
        .route("/calendar", get(calendar_demo))
        .route("/dist/{*filename}", get(serve_static))
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
    async fn test_index_page() {
        let state = AppState::default();
        let app = create_app(state);

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8(body.to_vec()).unwrap();

        assert!(html.contains("User Management"));
        assert!(html.contains("Add New User"));
    }

    #[tokio::test]
    async fn test_list_users_empty() {
        let state = AppState::default();
        let app = create_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/users")
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
    async fn test_create_and_get_user() {
        let state = AppState::default();
        let app = create_app(state);

        // Create a user
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/users")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from("name=John&email=john@example.com"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let user: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(user["name"], "John");
        assert_eq!(user["email"], "john@example.com");

        // Get the user
        let user_id = user["id"].as_str().unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/users/{user_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_nonexistent_user() {
        let state = AppState::default();
        let app = create_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/users/00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_user() {
        let state = AppState::default();
        let app = create_app(state);

        // Create a user first
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/users")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from("name=Jane&email=jane@example.com"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let user: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let user_id = user["id"].as_str().unwrap();

        // Delete the user
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/users/{user_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify the user is gone
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/users/{user_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
