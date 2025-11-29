use askama::Template;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Form, Json,
};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{CreateUser, User},
    state::AppState,
};

/// Template for rendering a single user row (used by HTMX).
#[derive(Template)]
#[template(path = "partials/user_row.html")]
struct UserRowTemplate<'a> {
    user: &'a User,
}

/// Check if the request is from HTMX.
fn is_htmx_request(headers: &HeaderMap) -> bool {
    headers.contains_key("HX-Request")
}

/// List all users (GET /api/users).
///
/// Returns JSON array of all users.
pub async fn list_users(State(state): State<AppState>) -> impl IntoResponse {
    let users: Vec<User> = state
        .users
        .read()
        .expect("Failed to acquire read lock")
        .values()
        .cloned()
        .collect();

    Json(users)
}

/// Create a new user (POST /api/users).
///
/// Accepts form data and returns either HTML (for HTMX) or JSON.
pub async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(payload): Form<CreateUser>,
) -> Result<Response, AppError> {
    let user = User::new(payload.name, payload.email);

    state
        .users
        .write()
        .expect("Failed to acquire write lock")
        .insert(user.id, user.clone());

    tracing::info!(user_id = %user.id, "Created new user");

    // Return HTML for HTMX requests, JSON otherwise
    if is_htmx_request(&headers) {
        let template = UserRowTemplate { user: &user };
        let html = template
            .render()
            .map_err(|e| anyhow::anyhow!("Template error: {e}"))?;
        Ok(Html(html).into_response())
    } else {
        Ok((StatusCode::CREATED, Json(user)).into_response())
    }
}

/// Get a single user by ID (GET /api/users/{id}).
pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<User>, StatusCode> {
    state
        .users
        .read()
        .expect("Failed to acquire read lock")
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Delete a user by ID (DELETE /api/users/{id}).
///
/// Returns empty response on success (HTMX will remove the row).
pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let removed = state
        .users
        .write()
        .expect("Failed to acquire write lock")
        .remove(&id);

    match removed {
        Some(user) => {
            tracing::info!(user_id = %id, user_name = %user.name, "Deleted user!");
            Ok(StatusCode::OK)
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}
