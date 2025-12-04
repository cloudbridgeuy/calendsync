use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
};
use tokio::sync::broadcast;
use uuid::Uuid;

use calendsync_ssr::SsrPool;

use crate::mock_data::generate_mock_entries;
use crate::models::User;

// Re-export core types for use in handlers
pub use calendsync_core::calendar::{Calendar, CalendarEntry, CalendarEvent};

/// A stored event with its ID for replay on reconnection.
#[derive(Clone, Debug)]
pub struct StoredEvent {
    pub id: u64,
    pub calendar_id: Uuid,
    pub event: CalendarEvent,
}

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

    /// Fixed demo calendar ID for predictable development URLs.
    /// Use: `/calendar/00000000-0000-0000-0000-000000000001`
    pub const DEMO_CALENDAR_ID: &'static str = "00000000-0000-0000-0000-000000000001";

    /// Create a new application state with demo data.
    pub fn with_demo_data() -> Self {
        let state = Self::new();

        // Use fixed UUID for predictable demo URLs
        let calendar_id =
            Uuid::parse_str(Self::DEMO_CALENDAR_ID).expect("Invalid demo calendar UUID constant");

        // Create default "Personal" calendar with fixed ID
        let default_calendar = Calendar::new("Personal", "#3B82F6")
            .with_id(calendar_id)
            .with_description("My personal calendar");

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
