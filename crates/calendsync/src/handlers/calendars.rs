use axum::{
    extract::{Path, State},
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

/// List all calendars (GET /api/calendars).
pub async fn list_calendars(State(state): State<AppState>) -> impl IntoResponse {
    let calendars: Vec<Calendar> = state
        .calendars
        .read()
        .expect("Failed to acquire read lock")
        .values()
        .cloned()
        .collect();

    Json(calendars)
}

/// Create a new calendar (POST /api/calendars).
pub async fn create_calendar(
    State(state): State<AppState>,
    Form(payload): Form<CreateCalendar>,
) -> Result<impl IntoResponse, StatusCode> {
    let calendar = payload.into_calendar();

    state
        .calendars
        .write()
        .expect("Failed to acquire write lock")
        .insert(calendar.id, calendar.clone());

    tracing::info!(calendar_id = %calendar.id, name = %calendar.name, "Created new calendar");

    Ok((StatusCode::CREATED, Json(calendar)))
}

/// Get a single calendar by ID (GET /api/calendars/{id}).
pub async fn get_calendar(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Calendar>, StatusCode> {
    state
        .calendars
        .read()
        .expect("Failed to acquire read lock")
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Update a calendar by ID (PUT /api/calendars/{id}).
pub async fn update_calendar(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Form(payload): Form<UpdateCalendar>,
) -> Result<Json<Calendar>, StatusCode> {
    let mut calendars = state
        .calendars
        .write()
        .expect("Failed to acquire write lock");

    let calendar = calendars.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;

    payload.apply_to(calendar);

    tracing::info!(calendar_id = %id, "Updated calendar");

    Ok(Json(calendar.clone()))
}

/// Delete a calendar by ID (DELETE /api/calendars/{id}).
///
/// Also deletes all entries belonging to this calendar.
pub async fn delete_calendar(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // Remove the calendar
    let removed = state
        .calendars
        .write()
        .expect("Failed to acquire write lock")
        .remove(&id);

    if removed.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Remove all entries belonging to this calendar
    let mut entries = state.entries.write().expect("Failed to acquire write lock");

    let entry_ids_to_remove: Vec<Uuid> = entries
        .iter()
        .filter(|(_, entry)| entry.calendar_id == id)
        .map(|(entry_id, _)| *entry_id)
        .collect();

    for entry_id in entry_ids_to_remove {
        entries.remove(&entry_id);
    }

    tracing::info!(calendar_id = %id, "Deleted calendar and its entries");

    Ok(StatusCode::OK)
}
