use std::collections::BTreeMap;

use axum::{
    extract::{rejection::FormRejection, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Form, Json,
};
use serde::Deserialize;
use uuid::Uuid;

use calendsync_core::calendar::{filter_entries, CalendarEntry, CalendarEvent, EntryKind};

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

// ============================================================================
// Calendar entries API (ServerDay[] format for React calendar)
// ============================================================================

/// Convert CalendarEntry to the ServerEntry format expected by the frontend.
pub fn entry_to_server_entry(entry: &CalendarEntry) -> serde_json::Value {
    let (kind, completed, is_multi_day, is_all_day, is_timed, is_task) = match &entry.kind {
        EntryKind::AllDay => ("all-day", false, false, true, false, false),
        EntryKind::Timed { .. } => ("timed", false, false, false, true, false),
        EntryKind::Task { completed } => ("task", *completed, false, false, false, true),
        EntryKind::MultiDay { .. } => ("multi-day", false, true, false, false, false),
    };

    let start_time = entry
        .kind
        .start_time()
        .map(|t| t.format("%H:%M").to_string());
    let end_time = entry.kind.end_time().map(|t| t.format("%H:%M").to_string());
    let multi_day_start = entry
        .kind
        .multi_day_start()
        .map(|d| d.format("%b %d").to_string());
    let multi_day_end = entry
        .kind
        .multi_day_end()
        .map(|d| d.format("%b %d").to_string());
    let multi_day_start_date = entry.kind.multi_day_start().map(|d| d.to_string());
    let multi_day_end_date = entry.kind.multi_day_end().map(|d| d.to_string());

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
        "date": entry.date.to_string(),
        "startTime": start_time,
        "endTime": end_time,
        "multiDayStart": multi_day_start,
        "multiDayEnd": multi_day_end,
        "multiDayStartDate": multi_day_start_date,
        "multiDayEndDate": multi_day_end_date,
    })
}

/// Group entries by date into ServerDay format for a date range.
/// Creates entries for all dates in the range, even if they have no entries.
fn entries_to_server_days(
    entries: &[&CalendarEntry],
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
) -> Vec<serde_json::Value> {
    // Build a map of entries by date
    let mut days_map: BTreeMap<chrono::NaiveDate, Vec<serde_json::Value>> = BTreeMap::new();

    // Initialize all dates in the range with empty vectors
    let mut current = start;
    while current <= end {
        days_map.insert(current, Vec::new());
        current += chrono::Duration::days(1);
    }

    // Add entries to their respective dates
    for entry in entries {
        if entry.date >= start && entry.date <= end {
            let server_entry = entry_to_server_entry(entry);
            days_map.entry(entry.date).or_default().push(server_entry);
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

/// Query parameters for the calendar entries API.
#[derive(serde::Deserialize)]
pub struct CalendarEntriesQuery {
    /// Calendar ID to fetch entries for.
    pub calendar_id: Uuid,
    /// Center date (ISO 8601: YYYY-MM-DD)
    pub highlighted_day: chrono::NaiveDate,
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

/// API handler for fetching calendar entries in ServerDay[] format.
/// Used by the React calendar client for data fetching.
///
/// GET /api/entries/calendar?calendar_id=...&highlighted_day=...&before=3&after=3
///
/// NOTE: This generates mock entries on-the-fly for the requested date range.
/// In production, this would query a database.
#[axum::debug_handler]
pub async fn list_calendar_entries(
    State(_state): State<AppState>,
    Query(query): Query<CalendarEntriesQuery>,
) -> Json<Vec<serde_json::Value>> {
    let start = query.highlighted_day - chrono::Duration::days(query.before);
    let end = query.highlighted_day + chrono::Duration::days(query.after);

    // Generate mock entries for the requested date range
    let entries = crate::mock_data::generate_mock_entries(query.calendar_id, query.highlighted_day);
    let filtered: Vec<&CalendarEntry> =
        filter_entries(&entries, Some(query.calendar_id), Some(start), Some(end));
    let days = entries_to_server_days(&filtered, start, end);

    Json(days)
}
