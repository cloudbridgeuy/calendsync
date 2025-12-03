//! SSE events handler for real-time calendar updates.

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
};
use chrono::{Days, Local, NaiveDate, NaiveTime};
use rand::prelude::*;
use rand::rngs::StdRng;
use uuid::Uuid;

use calendsync_core::calendar::{CalendarEntry, EntryKind};

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

        // Create a thread-safe RNG
        let mut rng = StdRng::from_os_rng();

        // Track session start for max duration (1 hour)
        let session_start = std::time::Instant::now();
        let max_duration = Duration::from_secs(3600);

        // Then generate random events every 3-5 seconds
        loop {
            // Check if we've exceeded max session duration
            if session_start.elapsed() > max_duration {
                tracing::info!("SSE session exceeded max duration, closing");
                break;
            }

            // Random delay between 3 and 5 seconds, or shutdown
            let delay_ms = rng.random_range(3000..=5000);
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {}
                _ = shutdown_rx.recv() => {
                    tracing::info!("SSE session received shutdown signal");
                    break;
                }
            }

            // Generate a random event
            let event = generate_random_event(&state, calendar_id, &mut rng);

            // Add to history and get event ID
            let event_id = state.add_event(calendar_id, event.clone());

            // Convert to SSE event
            let event_data = serde_json::to_string(&event).unwrap_or_default();
            let event_type = match &event {
                CalendarEvent::EntryAdded { .. } => "entry_added",
                CalendarEvent::EntryUpdated { .. } => "entry_updated",
                CalendarEvent::EntryDeleted { .. } => "entry_deleted",
            };

            yield Ok(Event::default()
                .id(event_id.to_string())
                .event(event_type)
                .data(event_data));
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Generate a random calendar event for simulation.
fn generate_random_event(state: &AppState, calendar_id: Uuid, rng: &mut StdRng) -> CalendarEvent {
    let today = Local::now().date_naive();

    // Random date within ±7 days of today
    let day_offset: i64 = rng.random_range(-7..=7);
    let date = if day_offset >= 0 {
        today
            .checked_add_days(Days::new(day_offset as u64))
            .unwrap_or(today)
    } else {
        today
            .checked_sub_days(Days::new((-day_offset) as u64))
            .unwrap_or(today)
    };
    let date_str = date.format("%Y-%m-%d").to_string();

    // Decide what type of event to generate
    let event_type: u8 = rng.random_range(0..=2);

    match event_type {
        0 => {
            // Add new entry
            let entry = generate_random_entry(calendar_id, date, rng);
            // Also add to state entries
            if let Ok(mut entries) = state.entries.write() {
                entries.insert(entry.id, entry.clone());
            }
            CalendarEvent::EntryAdded {
                entry,
                date: date_str,
            }
        }
        1 => {
            // Update existing entry (if any exist)
            if let Ok(entries) = state.entries.read() {
                let entry_ids: Vec<_> = entries.keys().copied().collect();
                if !entry_ids.is_empty() {
                    let idx = rng.random_range(0..entry_ids.len());
                    let entry_id = entry_ids[idx];
                    drop(entries);

                    // Modify the entry
                    if let Ok(mut entries) = state.entries.write() {
                        if let Some(entry) = entries.get_mut(&entry_id) {
                            // Update the title
                            entry.title = format!("{} (updated)", entry.title);
                            let entry_date = entry.date.format("%Y-%m-%d").to_string();
                            return CalendarEvent::EntryUpdated {
                                entry: entry.clone(),
                                date: entry_date,
                            };
                        }
                    }
                }
            }
            // Fallback to adding if no entries exist
            let entry = generate_random_entry(calendar_id, date, rng);
            if let Ok(mut entries) = state.entries.write() {
                entries.insert(entry.id, entry.clone());
            }
            CalendarEvent::EntryAdded {
                entry,
                date: date_str,
            }
        }
        _ => {
            // Delete existing entry (if any exist)
            if let Ok(mut entries) = state.entries.write() {
                let entry_ids: Vec<_> = entries.keys().copied().collect();
                if !entry_ids.is_empty() {
                    let idx = rng.random_range(0..entry_ids.len());
                    let entry_id = entry_ids[idx];
                    if let Some(removed) = entries.remove(&entry_id) {
                        let entry_date = removed.date.format("%Y-%m-%d").to_string();
                        return CalendarEvent::EntryDeleted {
                            entry_id,
                            date: entry_date,
                        };
                    }
                }
            }
            // Fallback to adding if no entries to delete
            let entry = generate_random_entry(calendar_id, date, rng);
            if let Ok(mut entries) = state.entries.write() {
                entries.insert(entry.id, entry.clone());
            }
            CalendarEvent::EntryAdded {
                entry,
                date: date_str,
            }
        }
    }
}

/// Generate a random calendar entry.
fn generate_random_entry(calendar_id: Uuid, date: NaiveDate, rng: &mut StdRng) -> CalendarEntry {
    let titles = [
        "Team Meeting",
        "Coffee with Sarah",
        "Project Review",
        "Lunch Break",
        "Call with Client",
        "Dentist Appointment",
        "Gym Session",
        "Book Club",
        "Movie Night",
        "Grocery Shopping",
    ];

    let descriptions = [
        Some("Don't forget to prepare the agenda"),
        Some("At the usual place"),
        Some("Q4 planning discussion"),
        None,
        Some("Discuss contract renewal"),
        None,
        Some("Leg day!"),
        Some("Currently reading: The Midnight Library"),
        None,
        Some("Need milk, eggs, bread"),
    ];

    let locations = [
        Some("Conference Room A"),
        Some("Downtown Cafe"),
        Some("Zoom"),
        None,
        Some("Phone"),
        Some("Dr. Smith's Office"),
        Some("Fitness Center"),
        Some("Library"),
        Some("Cinema"),
        Some("Whole Foods"),
    ];

    let idx = rng.random_range(0..titles.len());

    // Random hour between 8 and 18
    let hour: u32 = rng.random_range(8..=18);
    let start_time = NaiveTime::from_hms_opt(hour, 0, 0)
        .unwrap_or_else(|| NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    let end_time = start_time + chrono::Duration::hours(1);

    // Random entry kind
    let kind_choice: u8 = rng.random_range(0..=9);
    let kind = match kind_choice {
        0..=1 => EntryKind::Task {
            completed: rng.random_bool(0.3),
        },
        2 => EntryKind::AllDay,
        _ => EntryKind::Timed {
            start: start_time,
            end: end_time,
        },
    };

    CalendarEntry {
        id: Uuid::new_v4(),
        calendar_id,
        title: titles[idx].to_string(),
        description: descriptions[idx].map(|s| s.to_string()),
        location: locations[idx].map(|s| s.to_string()),
        kind,
        date,
        color: None,
    }
}
