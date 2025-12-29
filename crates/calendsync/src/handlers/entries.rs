//! Entry CRUD handlers.
//!
//! These handlers use repository trait objects for database access.
//! Event publishing is handled by the cached repository decorator.

use std::collections::BTreeMap;

use axum::{
    extract::{rejection::FormRejection, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Form, Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use uuid::Uuid;

use calendsync_core::calendar::{CalendarEntry, EntryKind};
use calendsync_core::storage::{DateRange, RepositoryError};

use crate::{
    handlers::AppError,
    models::{CreateEntry, UpdateEntry},
    state::AppState,
};

/// Error response with message (for form validation errors).
fn error_response(status: StatusCode, message: impl Into<String>) -> (StatusCode, String) {
    let msg = message.into();
    tracing::warn!(status = %status, message = %msg, "API error");
    (status, msg)
}

/// Query parameters for listing entries.
#[derive(Debug, Deserialize)]
pub struct ListEntriesQuery {
    /// Filter by calendar ID (required)
    pub calendar_id: Uuid,
    /// Center date for React calendar (ISO 8601: YYYY-MM-DD)
    pub highlighted_day: Option<NaiveDate>,
    /// Number of days before highlighted_day (default: 365)
    #[serde(default = "default_before")]
    pub before: i64,
    /// Number of days after highlighted_day (default: 365)
    #[serde(default = "default_after")]
    pub after: i64,
}

fn default_before() -> i64 {
    365
}

fn default_after() -> i64 {
    365
}

/// List entries for a calendar (GET /api/entries).
///
/// Returns entries in ServerDay[] format for the React calendar.
///
/// Query parameters:
/// - `calendar_id`: Calendar ID (required)
/// - `highlighted_day`: Center date (defaults to today)
/// - `before`: Days before highlighted_day (default: 365)
/// - `after`: Days after highlighted_day (default: 365)
#[axum::debug_handler]
pub async fn list_entries(
    State(state): State<AppState>,
    Query(query): Query<ListEntriesQuery>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let highlighted = query
        .highlighted_day
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let start = highlighted - chrono::Duration::days(query.before);
    let end = highlighted + chrono::Duration::days(query.after);

    let date_range = DateRange::new(start, end)?;

    let entries = state
        .entry_repo
        .get_entries_by_calendar(query.calendar_id, date_range)
        .await?;

    let entry_refs: Vec<&CalendarEntry> = entries.iter().collect();
    let days = entries_to_server_days(&entry_refs, start, end);

    Ok(Json(days))
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
    let calendar = state
        .calendar_repo
        .get_calendar(payload.calendar_id)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if calendar.is_none() {
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

    // Create entry via repository (which handles cache invalidation and event publishing)
    state
        .entry_repo
        .create_entry(&entry)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!(entry_id = %entry.id, title = %entry.title, "Created new entry");

    Ok((StatusCode::CREATED, Json(entry_to_server_entry(&entry))))
}

/// Get a single entry by ID (GET /api/entries/{id}).
pub async fn get_entry(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CalendarEntry>, AppError> {
    let entry = state.entry_repo.get_entry(id).await?;

    match entry {
        Some(e) => Ok(Json(e)),
        None => Err(RepositoryError::NotFound {
            entity_type: "CalendarEntry",
            id: id.to_string(),
        }
        .into()),
    }
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

    // Get existing entry
    let existing = state
        .entry_repo
        .get_entry(id)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, format!("Entry {id} not found")))?;

    // Apply updates to a mutable copy
    let mut updated_entry = existing;
    payload.apply_to(&mut updated_entry);

    // Update via repository (which handles cache invalidation and event publishing)
    state
        .entry_repo
        .update_entry(&updated_entry)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!(entry_id = %id, "Updated entry");

    Ok(Json(entry_to_server_entry(&updated_entry)))
}

/// Delete an entry by ID (DELETE /api/entries/{id}).
pub async fn delete_entry(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    tracing::debug!(entry_id = %id, "Received delete entry request");

    // Delete via repository (which handles cache invalidation and event publishing)
    state.entry_repo.delete_entry(id).await?;

    tracing::info!(entry_id = %id, "Deleted entry");

    Ok(StatusCode::OK)
}

/// Toggle a task's completion status (PATCH /api/entries/{id}/toggle).
pub async fn toggle_entry(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::debug!(entry_id = %id, "Received toggle entry request");

    // Get existing entry
    let existing = state
        .entry_repo
        .get_entry(id)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, format!("Entry {id} not found")))?;

    // Toggle if it's a task
    let mut updated_entry = existing;
    match &mut updated_entry.kind {
        EntryKind::Task { completed } => {
            *completed = !*completed;
            tracing::info!(entry_id = %id, completed = %completed, "Toggled task");
        }
        _ => {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                format!("Entry {id} is not a task"),
            ));
        }
    }

    // Update via repository (which handles cache invalidation and event publishing)
    state
        .entry_repo
        .update_entry(&updated_entry)
        .await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(entry_to_server_entry(&updated_entry)))
}

// ============================================================================
// Helper functions for ServerDay[] format
// ============================================================================

/// Convert CalendarEntry to the ServerEntry format expected by the frontend.
pub fn entry_to_server_entry(entry: &CalendarEntry) -> serde_json::Value {
    let (kind, completed, is_multi_day, is_all_day, is_timed, is_task) = match &entry.kind {
        EntryKind::AllDay => ("all-day", false, false, true, false, false),
        EntryKind::Timed { .. } => ("timed", false, false, false, true, false),
        EntryKind::Task { completed } => ("task", *completed, false, false, false, true),
        EntryKind::MultiDay => ("multi-day", false, true, false, false, false),
    };

    let start_time = entry
        .kind
        .start_time()
        .map(|t| t.format("%H:%M").to_string());
    let end_time = entry.kind.end_time().map(|t| t.format("%H:%M").to_string());

    serde_json::json!({
        "id": entry.id.to_string(),
        "calendarId": entry.calendar_id.to_string(),
        "kind": kind,
        "completed": completed,
        "isMultiDay": is_multi_day,
        "isAllDay": is_all_day,
        "isTimed": is_timed,
        "isTask": is_task,
        "title": entry.title,
        "description": entry.description,
        "location": entry.location,
        "color": entry.color,
        "startDate": entry.start_date.to_string(),
        "endDate": entry.end_date.to_string(),
        "startTime": start_time,
        "endTime": end_time,
    })
}

/// Group entries by date into ServerDay format for a date range.
/// Creates entries for all dates in the range, even if they have no entries.
pub fn entries_to_server_days(
    entries: &[&CalendarEntry],
    start: NaiveDate,
    end: NaiveDate,
) -> Vec<serde_json::Value> {
    let mut days_map: BTreeMap<NaiveDate, Vec<serde_json::Value>> = BTreeMap::new();

    // Initialize all dates in the range
    let mut current = start;
    while current <= end {
        days_map.insert(current, Vec::new());
        current += chrono::Duration::days(1);
    }

    // Add entries - use start_date for grouping
    // (Frontend will expand multi-day entries)
    for entry in entries {
        if entry.start_date >= start && entry.start_date <= end {
            let server_entry = entry_to_server_entry(entry);
            days_map
                .entry(entry.start_date)
                .or_default()
                .push(server_entry);
        }
    }

    days_map
        .into_iter()
        .map(|(date, entries)| {
            serde_json::json!({
                "date": date.to_string(),
                "entries": entries,
            })
        })
        .collect()
}
