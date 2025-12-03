use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
};
use tokio::sync::broadcast;
use uuid::Uuid;

use calendsync_core::calendar::{Calendar, CalendarEntry};
use calendsync_ssr::SsrPool;

use crate::mock_data::generate_mock_entries;
use crate::models::User;

/// SSE event types for calendar updates.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum CalendarEvent {
    EntryAdded { entry: CalendarEntry, date: String },
    EntryUpdated { entry: CalendarEntry, date: String },
    EntryDeleted { entry_id: Uuid, date: String },
}

/// A stored event with its ID for replay on reconnection.
#[derive(Clone, Debug)]
pub struct StoredEvent {
    pub id: u64,
    pub calendar_id: Uuid,
    pub event: CalendarEvent,
}

/// Maximum number of events to keep in history for reconnection catch-up.
const MAX_EVENT_HISTORY: usize = 1000;

/// Shared application state.
///
/// This is cloned for each request handler and contains shared resources
/// like the user repository, calendars, and entries.
#[derive(Clone)]
pub struct AppState {
    /// In-memory user storage.
    pub users: Arc<RwLock<HashMap<Uuid, User>>>,
    /// In-memory calendar storage.
    pub calendars: Arc<RwLock<HashMap<Uuid, Calendar>>>,
    /// In-memory entry storage.
    pub entries: Arc<RwLock<HashMap<Uuid, CalendarEntry>>>,
    /// Event counter for generating unique event IDs.
    pub event_counter: Arc<AtomicU64>,
    /// Event history for SSE reconnection catch-up.
    pub event_history: Arc<RwLock<VecDeque<StoredEvent>>>,
    /// Shutdown signal sender for SSE connections.
    pub shutdown_tx: broadcast::Sender<()>,
    /// SSR worker pool for React server-side rendering.
    /// None when SSR is not initialized (e.g., in tests).
    pub ssr_pool: Option<Arc<SsrPool>>,
}

impl Default for AppState {
    fn default() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            calendars: Arc::new(RwLock::new(HashMap::new())),
            entries: Arc::new(RwLock::new(HashMap::new())),
            event_counter: Arc::new(AtomicU64::new(1)),
            event_history: Arc::new(RwLock::new(VecDeque::new())),
            shutdown_tx,
            ssr_pool: None,
        }
    }
}

impl AppState {
    /// Create a new application state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the SSR pool.
    pub fn with_ssr_pool(mut self, pool: SsrPool) -> Self {
        self.ssr_pool = Some(Arc::new(pool));
        self
    }

    /// Create a new application state with demo data.
    pub fn with_demo_data() -> Self {
        let state = Self::new();

        // Create default "Personal" calendar
        let default_calendar =
            Calendar::new("Personal", "#3B82F6").with_description("My personal calendar");

        let calendar_id = default_calendar.id;

        // Store the calendar
        state
            .calendars
            .write()
            .expect("Failed to acquire calendars write lock")
            .insert(default_calendar.id, default_calendar);

        // Generate and store demo entries
        let today = chrono::Local::now().date_naive();
        let entries = generate_mock_entries(calendar_id, today);

        {
            let mut entries_store = state
                .entries
                .write()
                .expect("Failed to acquire entries write lock");

            for entry in entries {
                entries_store.insert(entry.id, entry);
            }
        }

        state
    }

    /// Get the default calendar ID (first calendar, or None if empty).
    #[allow(dead_code)]
    pub fn default_calendar_id(&self) -> Option<Uuid> {
        self.calendars
            .read()
            .ok()
            .and_then(|calendars| calendars.keys().next().copied())
    }

    /// Add an event to the history and return its ID.
    pub fn add_event(&self, calendar_id: Uuid, event: CalendarEvent) -> u64 {
        let event_id = self.event_counter.fetch_add(1, Ordering::SeqCst);

        let stored = StoredEvent {
            id: event_id,
            calendar_id,
            event,
        };

        if let Ok(mut history) = self.event_history.write() {
            history.push_back(stored);
            // Keep history bounded
            while history.len() > MAX_EVENT_HISTORY {
                history.pop_front();
            }
        }

        event_id
    }

    /// Get events since a given event ID for a specific calendar.
    pub fn get_events_since(&self, calendar_id: Uuid, since_id: u64) -> Vec<StoredEvent> {
        self.event_history
            .read()
            .ok()
            .map(|history| {
                history
                    .iter()
                    .filter(|e| e.id > since_id && e.calendar_id == calendar_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the current event counter value.
    #[allow(dead_code)]
    pub fn current_event_id(&self) -> u64 {
        self.event_counter.load(Ordering::SeqCst)
    }

    /// Subscribe to shutdown signal.
    pub fn subscribe_shutdown(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Signal all SSE connections to shut down.
    pub fn signal_shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}
