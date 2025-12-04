use axum::{
    extract::{rejection::FormRejection, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Form, Json,
};
use serde::Deserialize;
use uuid::Uuid;

use calendsync_core::calendar::{filter_entries, CalendarEntry, CalendarEvent, EntryKind};

use super::calendar_react::entry_to_server_entry;
use crate::{
    models::{CreateEntry, UpdateEntry},
    state::AppState,
};

/// Error response with message
fn error_response(status: StatusCode, message: impl Into<String>) -> (StatusCode, String) {
    let msg = message.into();
    tracing::warn!(status = %status, message = %msg, "API error");
    (status, msg)
}

/// Query parameters for listing entries.
#[derive(Debug, Deserialize)]
pub struct ListEntriesQuery {
    /// Filter by calendar ID
    pub calendar_id: Option<Uuid>,
    /// Filter by start date (inclusive) - legacy parameter
    pub start: Option<chrono::NaiveDate>,
    /// Filter by end date (inclusive) - legacy parameter
    pub end: Option<chrono::NaiveDate>,
    /// Center date for React calendar (ISO 8601: YYYY-MM-DD)
    pub highlighted_day: Option<chrono::NaiveDate>,
    /// Number of days before highlighted_day (default: 3)
    pub before: Option<i64>,
    /// Number of days after highlighted_day (default: 3)
    pub after: Option<i64>,
}

/// List all entries (GET /api/entries).
///
/// Supports optional query parameters for filtering:
/// - `calendar_id`: Filter by calendar
/// - `start`: Filter by start date (inclusive) - legacy
/// - `end`: Filter by end date (inclusive) - legacy
/// - `highlighted_day`: Center date for React calendar
/// - `before`: Number of days before highlighted_day (default: 3)
/// - `after`: Number of days after highlighted_day (default: 3)
///
/// If `highlighted_day` is provided, `before` and `after` are used to calculate the date range.
/// Otherwise, falls back to `start` and `end` parameters.
pub async fn list_entries(
    State(state): State<AppState>,
    Query(query): Query<ListEntriesQuery>,
) -> impl IntoResponse {
    let entries_store = state.entries.read().expect("Failed to acquire read lock");

    let all_entries: Vec<CalendarEntry> = entries_store.values().cloned().collect();

    // Calculate date range
    let (start, end) = if let Some(highlighted) = query.highlighted_day {
        // Use highlighted_day with before/after
        let before_days = query.before.unwrap_or(3);
        let after_days = query.after.unwrap_or(3);
        let start = highlighted - chrono::Duration::days(before_days);
        let end = highlighted + chrono::Duration::days(after_days);
        (Some(start), Some(end))
    } else {
        // Fall back to legacy start/end parameters
        (query.start, query.end)
    };

    // Filter by calendar_id if provided
    let filtered: Vec<&CalendarEntry> = filter_entries(&all_entries, query.calendar_id, start, end);

    let result: Vec<CalendarEntry> = filtered.into_iter().cloned().collect();

    Json(result)
}

/// Create a new entry (POST /api/entries).
pub async fn create_entry(
    State(state): State<AppState>,
    form_result: Result<Form<CreateEntry>, FormRejection>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Handle form parsing errors
    let Form(payload) = form_result.map_err(|e| {
        let msg = format!("Failed to parse form: {e}");
        tracing::error!(error = %e, "Form parsing failed");
        error_response(StatusCode::BAD_REQUEST, msg)
    })?;

    tracing::debug!(payload = ?payload, "Received create entry request");

    // Verify the calendar exists
    let calendar_exists = state
        .calendars
        .read()
        .expect("Failed to acquire read lock")
        .contains_key(&payload.calendar_id);

    if !calendar_exists {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            format!("Calendar {} not found", payload.calendar_id),
        ));
    }

    let entry = payload.into_entry().ok_or_else(|| {
        error_response(
            StatusCode::BAD_REQUEST,
            "Invalid entry data: missing required fields for entry type",
        )
    })?;

    state
        .entries
        .write()
        .expect("Failed to acquire write lock")
        .insert(entry.id, entry.clone());

    // Publish SSE event for real-time updates
    state.publish_event(entry.calendar_id, CalendarEvent::entry_added(entry.clone()));

    tracing::info!(entry_id = %entry.id, title = %entry.title, "Created new entry");

    Ok((StatusCode::CREATED, Json(entry_to_server_entry(&entry))))
}

/// Get a single entry by ID (GET /api/entries/{id}).
pub async fn get_entry(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CalendarEntry>, (StatusCode, String)> {
    state
        .entries
        .read()
        .expect("Failed to acquire read lock")
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, format!("Entry {id} not found")))
}

/// Update an entry by ID (PUT /api/entries/{id}).
pub async fn update_entry(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    form_result: Result<Form<UpdateEntry>, FormRejection>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let Form(payload) = form_result.map_err(|e| {
        let msg = format!("Failed to parse form: {e}");
        tracing::error!(error = %e, "Form parsing failed");
        error_response(StatusCode::BAD_REQUEST, msg)
    })?;

    tracing::debug!(entry_id = %id, payload = ?payload, "Received update entry request");

    // Update entry and get a clone for the response and event
    let updated_entry = {
        let mut entries = state.entries.write().expect("Failed to acquire write lock");

        let entry = entries.get_mut(&id).ok_or_else(|| {
            error_response(StatusCode::NOT_FOUND, format!("Entry {id} not found"))
        })?;

        payload.apply_to(entry);
        entry.clone()
    }; // Lock is released here

    // Publish SSE event for real-time updates
    state.publish_event(
        updated_entry.calendar_id,
        CalendarEvent::entry_updated(updated_entry.clone()),
    );

    tracing::info!(entry_id = %id, "Updated entry");

    Ok(Json(entry_to_server_entry(&updated_entry)))
}

/// Delete an entry by ID (DELETE /api/entries/{id}).
pub async fn delete_entry(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    tracing::debug!(entry_id = %id, "Received delete entry request");

    let removed = state
        .entries
        .write()
        .expect("Failed to acquire write lock")
        .remove(&id);

    match removed {
        Some(entry) => {
            // Publish SSE event for real-time updates
            state.publish_event(
                entry.calendar_id,
                CalendarEvent::entry_deleted(entry.id, entry.date),
            );

            tracing::info!(entry_id = %id, title = %entry.title, "Deleted entry");
            Ok(StatusCode::OK)
        }
        None => Err(error_response(
            StatusCode::NOT_FOUND,
            format!("Entry {id} not found"),
        )),
    }
}

/// Toggle a task's completion status (PATCH /api/entries/{id}/toggle).
pub async fn toggle_entry(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::debug!(entry_id = %id, "Received toggle entry request");

    // Toggle entry and get a clone for the response and event
    let toggled_entry = {
        let mut entries = state.entries.write().expect("Failed to acquire write lock");

        let entry = entries.get_mut(&id).ok_or_else(|| {
            error_response(StatusCode::NOT_FOUND, format!("Entry {id} not found"))
        })?;

        // Only toggle if it's a task
        match &mut entry.kind {
            EntryKind::Task { completed } => {
                *completed = !*completed;
                tracing::info!(entry_id = %id, completed = %completed, "Toggled task");
                Ok(entry.clone())
            }
            _ => Err(error_response(
                StatusCode::BAD_REQUEST,
                format!("Entry {id} is not a task"),
            )),
        }
    }?; // Lock is released here

    // Publish SSE event for real-time updates
    state.publish_event(
        toggled_entry.calendar_id,
        CalendarEvent::entry_updated(toggled_entry.clone()),
    );

    Ok(Json(entry_to_server_entry(&toggled_entry)))
}
