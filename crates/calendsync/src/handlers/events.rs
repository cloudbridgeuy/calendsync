//! SSE events handler for real-time calendar updates.

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
};
use uuid::Uuid;

use crate::state::{AppState, CalendarEvent};

/// Query parameters for the SSE events endpoint.
#[derive(Debug, serde::Deserialize)]
pub struct EventsQuery {
    /// Calendar ID to subscribe to.
    pub calendar_id: Uuid,
    /// Last event ID received (for reconnection catch-up).
    pub last_event_id: Option<u64>,
}

/// SSE endpoint for calendar events.
///
/// Returns a stream of Server-Sent Events for the specified calendar.
/// If `last_event_id` is provided, sends missed events first before streaming new ones.
pub async fn events_sse(
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let calendar_id = query.calendar_id;
    let last_event_id = query.last_event_id.unwrap_or(0);

    // Subscribe to shutdown signal
    let mut shutdown_rx = state.subscribe_shutdown();

    // Create the stream
    let stream = async_stream::stream! {
        // First, send any missed events since last_event_id
        let missed_events = state.get_events_since(calendar_id, last_event_id);
        for stored in missed_events {
            let event_data = serde_json::to_string(&stored.event).unwrap_or_default();
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

        // Track session start for max duration (1 hour)
        let session_start = std::time::Instant::now();
        let max_duration = Duration::from_secs(3600);

        // Keep connection alive, waiting for real events or shutdown
        loop {
            // Check if we've exceeded max session duration
            if session_start.elapsed() > max_duration {
                tracing::info!("SSE session exceeded max duration, closing");
                break;
            }

            // Wait for shutdown signal (keep-alive is handled by Sse::keep_alive)
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    // Periodic check for max duration
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("SSE session received shutdown signal");
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
