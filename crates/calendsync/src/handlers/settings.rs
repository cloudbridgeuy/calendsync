//! Calendar settings handler.
//!
//! Handles persisting per-user, per-calendar display settings.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use calendsync_core::calendar::CalendarSettings;

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use axum::response::{IntoResponse, Response};

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use calendsync_auth::CurrentUser;

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use super::authz::require_read_access;

use crate::state::AppState;

/// Error response with message.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
fn error_response(status: StatusCode, message: impl Into<String>) -> (StatusCode, String) {
    let msg = message.into();
    tracing::warn!(status = %status, message = %msg, "API error");
    (status, msg)
}

// ============================================================================
// Update Settings
// ============================================================================

/// Update calendar settings for the current user (PUT /api/calendars/{id}/settings) - with auth.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
pub async fn update_settings(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(calendar_id): Path<Uuid>,
    Json(settings): Json<CalendarSettings>,
) -> Result<StatusCode, Response> {
    let auth = state.auth.as_ref().expect("Auth state required");
    require_read_access(auth, calendar_id, user.id)
        .await
        .map_err(IntoResponse::into_response)?;

    update_settings_impl(&state, calendar_id, user.id, settings)
        .await
        .map_err(IntoResponse::into_response)
}

/// Update calendar settings (PUT /api/calendars/{id}/settings) - no auth.
///
/// Without auth, settings are not persisted since there is no user identity.
/// Returns 200 OK as a no-op to avoid frontend errors.
#[cfg(not(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock")))]
pub async fn update_settings(
    State(_state): State<AppState>,
    Path(_calendar_id): Path<Uuid>,
    Json(_settings): Json<CalendarSettings>,
) -> StatusCode {
    // Without auth, there's no user to associate settings with.
    // Accept the request silently so the frontend doesn't error.
    StatusCode::OK
}

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
async fn update_settings_impl(
    state: &AppState,
    calendar_id: Uuid,
    user_id: Uuid,
    settings: CalendarSettings,
) -> Result<StatusCode, (StatusCode, String)> {
    tracing::debug!(
        calendar_id = %calendar_id,
        user_id = %user_id,
        ?settings,
        "Received update settings request"
    );

    state
        .settings_repo
        .upsert_settings(calendar_id, user_id, &settings)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::debug!(
        calendar_id = %calendar_id,
        user_id = %user_id,
        "Updated calendar settings"
    );

    Ok(StatusCode::OK)
}
