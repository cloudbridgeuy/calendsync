//! SSE events handler for real-time calendar updates.

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
};
use uuid::Uuid;

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use axum::response::{IntoResponse, Response};

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use calendsync_auth::CurrentUser;

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use super::authz::require_read_access;

use super::entries::entry_to_server_entry;
use crate::state::{AppState, CalendarEvent};

/// Serialize a CalendarEvent to JSON, transforming entries to ServerEntry format.
fn serialize_event(event: &CalendarEvent) -> String {
    match event {
        CalendarEvent::EntryAdded { entry, date } => {
            let server_entry = entry_to_server_entry(entry);
            serde_json::json!({
                "entry": server_entry,
                "date": date,
            })
            .to_string()
        }
        CalendarEvent::EntryUpdated { entry, date } => {
            let server_entry = entry_to_server_entry(entry);
            serde_json::json!({
                "entry": server_entry,
                "date": date,
            })
            .to_string()
        }
        CalendarEvent::EntryDeleted { entry_id, date } => serde_json::json!({
            "entry_id": entry_id,
            "date": date,
        })
        .to_string(),
    }
}

/// Query parameters for the SSE events endpoint.
#[derive(Debug, serde::Deserialize)]
pub struct EventsQuery {
    /// Calendar ID to subscribe to.
    pub calendar_id: Uuid,
    /// Last event ID received (for reconnection catch-up).
    pub last_event_id: Option<u64>,
}

/// SSE endpoint for calendar events - with auth.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
pub async fn events_sse(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, Response> {
    let auth = state.auth.as_ref().expect("Auth state required");
    require_read_access(auth, query.calendar_id, user.id)
        .await
        .map_err(IntoResponse::into_response)?;

    Ok(events_sse_impl(state, query))
}

/// SSE endpoint for calendar events - no auth.
#[cfg(not(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock")))]
pub async fn events_sse(
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    events_sse_impl(state, query)
}

fn events_sse_impl(
    state: AppState,
    query: EventsQuery,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let calendar_id = query.calendar_id;
    let last_event_id = query.last_event_id.unwrap_or(0);

    state.ensure_event_listener(calendar_id);

    let mut shutdown_rx = state.subscribe_shutdown();
    let oldest_event_id = state.oldest_event_id();

    let stream = async_stream::stream! {
        if last_event_id > 0 && last_event_id < oldest_event_id {
            yield Ok(Event::default()
                .event("refresh_required")
                .data(serde_json::json!({
                    "reason": "event_history_gap",
                    "message": "Please refresh entries from the server"
                }).to_string()));
        }

        let mut current_event_id = last_event_id;

        let missed_events = state.get_events_since(calendar_id, current_event_id);
        for stored in missed_events {
            current_event_id = stored.id;
            let event_data = serialize_event(&stored.event);
            let event_type = match &stored.event {
                CalendarEvent::EntryAdded { .. } => "entry_added",
                CalendarEvent::EntryUpdated { .. } => "entry_updated",
                CalendarEvent::EntryDeleted { .. } => "entry_deleted",
            };

            yield Ok(Event::default()
                .id(stored.id.to_string())
                .event(event_type)
                .data(event_data));
        }

        let session_start = std::time::Instant::now();
        let max_duration = Duration::from_secs(3600);
        let poll_interval = Duration::from_millis(100);

        loop {
            if session_start.elapsed() > max_duration {
                tracing::info!("SSE session exceeded max duration, closing");
                break;
            }

            let new_events = state.get_events_since(calendar_id, current_event_id);
            for stored in new_events {
                current_event_id = stored.id;
                let event_data = serialize_event(&stored.event);
                let event_type = match &stored.event {
                    CalendarEvent::EntryAdded { .. } => "entry_added",
                    CalendarEvent::EntryUpdated { .. } => "entry_updated",
                    CalendarEvent::EntryDeleted { .. } => "entry_deleted",
                };

                yield Ok(Event::default()
                    .id(stored.id.to_string())
                    .event(event_type)
                    .data(event_data));
            }

            tokio::select! {
                _ = tokio::time::sleep(poll_interval) => {}
                _ = shutdown_rx.recv() => {
                    tracing::info!("SSE session received shutdown signal");
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
