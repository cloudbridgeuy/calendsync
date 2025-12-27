//! Calendar CRUD handlers.

use axum::{
    extract::{rejection::FormRejection, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Form, Json,
};
use uuid::Uuid;

use calendsync_core::calendar::Calendar;

use crate::{
    models::{CreateCalendar, UpdateCalendar},
    state::AppState,
};

/// Error response with message (for form validation errors).
fn error_response(status: StatusCode, message: impl Into<String>) -> (StatusCode, String) {
    let msg = message.into();
    tracing::warn!(status = %status, message = %msg, "API error");
    (status, msg)
}

/// Create a new calendar (POST /api/calendars).
pub async fn create_calendar(
    State(state): State<AppState>,
    form_result: Result<Form<CreateCalendar>, FormRejection>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let Form(payload) = form_result.map_err(|e| {
        let msg = format!("Failed to parse form: {e}");
        tracing::error!(error = %e, "Form parsing failed");
        error_response(StatusCode::BAD_REQUEST, msg)
    })?;

    tracing::debug!(payload = ?payload, "Received create calendar request");

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
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!(calendar_id = %calendar.id, name = %calendar.name, "Created new calendar");

    Ok((StatusCode::CREATED, Json(calendar)))
}

/// Get a calendar by ID (GET /api/calendars/{id}).
pub async fn get_calendar(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
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

/// Update a calendar (PUT /api/calendars/{id}).
pub async fn update_calendar(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    form_result: Result<Form<UpdateCalendar>, FormRejection>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let Form(payload) = form_result.map_err(|e| {
        let msg = format!("Failed to parse form: {e}");
        tracing::error!(error = %e, "Form parsing failed");
        error_response(StatusCode::BAD_REQUEST, msg)
    })?;

    tracing::debug!(calendar_id = %id, payload = ?payload, "Received update calendar request");

    // Get existing calendar
    let mut calendar = state
        .calendar_repo
        .get_calendar(id)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "Calendar not found"))?;

    // Apply updates
    if let Some(name) = payload.name {
        calendar.name = name;
    }
    if let Some(color) = payload.color {
        calendar.color = color;
    }
    if let Some(description) = payload.description {
        calendar.description = Some(description);
    }

    // Update via repository
    state
        .calendar_repo
        .update_calendar(&calendar)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!(calendar_id = %id, "Updated calendar");

    Ok(Json(calendar))
}

/// Delete a calendar (DELETE /api/calendars/{id}).
pub async fn delete_calendar(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    tracing::debug!(calendar_id = %id, "Received delete calendar request");

    // Check if calendar exists
    let exists = state
        .calendar_repo
        .get_calendar(id)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_some();

    if !exists {
        return Err(error_response(StatusCode::NOT_FOUND, "Calendar not found"));
    }

    // Delete via repository
    state
        .calendar_repo
        .delete_calendar(id)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!(calendar_id = %id, "Deleted calendar");

    Ok(StatusCode::OK)
}
