//! Calendar CRUD handlers.

use axum::{
    extract::{rejection::FormRejection, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Form, Json,
};
use uuid::Uuid;

use calendsync_core::calendar::Calendar;

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use calendsync_core::calendar::CalendarRole;

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use serde::Serialize;

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use axum::response::Response;

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use calendsync_core::calendar::CalendarMembership;

use crate::{
    models::{CreateCalendar, UpdateCalendar},
    state::AppState,
};

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use calendsync_auth::CurrentUser;

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use super::authz::{require_admin_access, require_read_access, require_write_access};

/// Error response with message (for form validation errors).
fn error_response(status: StatusCode, message: impl Into<String>) -> (StatusCode, String) {
    let msg = message.into();
    tracing::warn!(status = %status, message = %msg, "API error");
    (status, msg)
}

/// Calendar with the user's membership role.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
#[derive(Serialize)]
pub struct CalendarWithRole {
    #[serde(flatten)]
    pub calendar: Calendar,
    pub role: CalendarRole,
}

// ============================================================================
// List My Calendars
// ============================================================================

/// List all calendars the current user has access to (GET /api/calendars/me).
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
pub async fn list_my_calendars(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<CalendarWithRole>>, Response> {
    list_my_calendars_impl(&state, user.id)
        .await
        .map_err(IntoResponse::into_response)
}

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
async fn list_my_calendars_impl(
    state: &AppState,
    user_id: Uuid,
) -> Result<Json<Vec<CalendarWithRole>>, (StatusCode, String)> {
    let auth = state
        .auth
        .as_ref()
        .expect("Auth state required when auth feature enabled");
    let calendars = auth
        .memberships
        .get_calendars_for_user(user_id)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::debug!(
        user_id = %user_id,
        calendar_count = calendars.len(),
        "Listed calendars for user"
    );

    Ok(Json(
        calendars
            .into_iter()
            .map(|(calendar, role)| CalendarWithRole { calendar, role })
            .collect(),
    ))
}

// ============================================================================
// Create Calendar
// ============================================================================

/// Create a new calendar (POST /api/calendars) - with auth.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
pub async fn create_calendar(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    form_result: Result<Form<CreateCalendar>, FormRejection>,
) -> Result<impl IntoResponse, Response> {
    let Form(payload) = form_result.map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            format!("Failed to parse form: {e}"),
        )
        .into_response()
    })?;

    tracing::debug!(user_id = %user.id, payload = ?payload, "Received create calendar request");

    // Build calendar from payload
    let mut calendar = Calendar::new(&payload.name, &payload.color);
    if let Some(desc) = payload.description {
        calendar = calendar.with_description(desc);
    }

    // Create via repository
    state
        .calendar_repo
        .create_calendar(&calendar)
        .await
        .map_err(|e| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        })?;

    // Create owner membership for the creating user
    let auth = state
        .auth
        .as_ref()
        .expect("Auth state required when auth feature enabled");
    let membership = CalendarMembership::owner(calendar.id, user.id);
    auth.memberships
        .create_membership(&membership)
        .await
        .map_err(|e| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        })?;

    tracing::info!(
        calendar_id = %calendar.id,
        user_id = %user.id,
        name = %calendar.name,
        "Created new calendar with owner membership"
    );

    Ok((StatusCode::CREATED, Json(calendar)))
}

/// Create a new calendar (POST /api/calendars) - no auth.
#[cfg(not(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock")))]
pub async fn create_calendar(
    State(state): State<AppState>,
    form_result: Result<Form<CreateCalendar>, FormRejection>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let Form(payload) = form_result.map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            format!("Failed to parse form: {e}"),
        )
    })?;

    tracing::debug!(payload = ?payload, "Received create calendar request");

    let mut calendar = Calendar::new(&payload.name, &payload.color);
    if let Some(desc) = payload.description {
        calendar = calendar.with_description(desc);
    }

    state
        .calendar_repo
        .create_calendar(&calendar)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!(calendar_id = %calendar.id, name = %calendar.name, "Created new calendar");

    Ok((StatusCode::CREATED, Json(calendar)))
}

// ============================================================================
// Get Calendar
// ============================================================================

/// Get a calendar by ID (GET /api/calendars/{id}) - with auth.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
pub async fn get_calendar(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, Response> {
    let auth = state.auth.as_ref().expect("Auth state required");
    require_read_access(auth, id, user.id)
        .await
        .map_err(IntoResponse::into_response)?;

    get_calendar_impl(&state, id)
        .await
        .map_err(IntoResponse::into_response)
}

/// Get a calendar by ID (GET /api/calendars/{id}) - no auth.
#[cfg(not(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock")))]
pub async fn get_calendar(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    get_calendar_impl(&state, id).await
}

async fn get_calendar_impl(
    state: &AppState,
    id: Uuid,
) -> Result<Json<Calendar>, (StatusCode, String)> {
    tracing::debug!(calendar_id = %id, "Received get calendar request");

    let calendar = state
        .calendar_repo
        .get_calendar(id)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match calendar {
        Some(cal) => {
            tracing::debug!(calendar_id = %id, "Found calendar");
            Ok(Json(cal))
        }
        None => {
            tracing::debug!(calendar_id = %id, "Calendar not found");
            Err(error_response(StatusCode::NOT_FOUND, "Calendar not found"))
        }
    }
}

// ============================================================================
// Update Calendar
// ============================================================================

/// Update a calendar (PUT /api/calendars/{id}) - with auth.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
pub async fn update_calendar(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    form_result: Result<Form<UpdateCalendar>, FormRejection>,
) -> Result<impl IntoResponse, Response> {
    let auth = state.auth.as_ref().expect("Auth state required");
    require_write_access(auth, id, user.id)
        .await
        .map_err(IntoResponse::into_response)?;

    let Form(payload) = form_result.map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            format!("Failed to parse form: {e}"),
        )
        .into_response()
    })?;

    update_calendar_impl(&state, id, payload)
        .await
        .map_err(IntoResponse::into_response)
}

/// Update a calendar (PUT /api/calendars/{id}) - no auth.
#[cfg(not(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock")))]
pub async fn update_calendar(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    form_result: Result<Form<UpdateCalendar>, FormRejection>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let Form(payload) = form_result.map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            format!("Failed to parse form: {e}"),
        )
    })?;

    update_calendar_impl(&state, id, payload).await
}

async fn update_calendar_impl(
    state: &AppState,
    id: Uuid,
    payload: UpdateCalendar,
) -> Result<Json<Calendar>, (StatusCode, String)> {
    tracing::debug!(calendar_id = %id, payload = ?payload, "Received update calendar request");

    let mut calendar = state
        .calendar_repo
        .get_calendar(id)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "Calendar not found"))?;

    if let Some(name) = payload.name {
        calendar.name = name;
    }
    if let Some(color) = payload.color {
        calendar.color = color;
    }
    if let Some(description) = payload.description {
        calendar.description = Some(description);
    }

    state
        .calendar_repo
        .update_calendar(&calendar)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!(calendar_id = %id, "Updated calendar");

    Ok(Json(calendar))
}

// ============================================================================
// Delete Calendar
// ============================================================================

/// Delete a calendar (DELETE /api/calendars/{id}) - with auth.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
pub async fn delete_calendar(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, Response> {
    let auth = state.auth.as_ref().expect("Auth state required");
    require_admin_access(auth, id, user.id)
        .await
        .map_err(IntoResponse::into_response)?;

    delete_calendar_impl(&state, id)
        .await
        .map_err(IntoResponse::into_response)
}

/// Delete a calendar (DELETE /api/calendars/{id}) - no auth.
#[cfg(not(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock")))]
pub async fn delete_calendar(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    delete_calendar_impl(&state, id).await
}

async fn delete_calendar_impl(
    state: &AppState,
    id: Uuid,
) -> Result<StatusCode, (StatusCode, String)> {
    tracing::debug!(calendar_id = %id, "Received delete calendar request");

    let exists = state
        .calendar_repo
        .get_calendar(id)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_some();

    if !exists {
        return Err(error_response(StatusCode::NOT_FOUND, "Calendar not found"));
    }

    state
        .calendar_repo
        .delete_calendar(id)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!(calendar_id = %id, "Deleted calendar");

    Ok(StatusCode::OK)
}
