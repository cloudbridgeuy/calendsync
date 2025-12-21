//! Calendar CRUD handlers.

use axum::{
    extract::{rejection::FormRejection, State},
    http::StatusCode,
    response::IntoResponse,
    Form, Json,
};

use calendsync_core::calendar::Calendar;

use crate::{models::CreateCalendar, state::AppState};

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
