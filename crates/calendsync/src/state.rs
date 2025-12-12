use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
};
use tokio::sync::{broadcast, RwLock as TokioRwLock};
use uuid::Uuid;

use calendsync_ssr::SsrPool;

use crate::mock_data::generate_mock_entries;

/// Build error message for dev mode error overlay.
#[derive(Clone, Debug)]
pub struct BuildError {
    pub error: String,
}

/// CSS reload message for dev mode CSS hot-swap.
#[derive(Clone, Debug)]
pub struct CssReload {
    pub filename: String,
}

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
/// like calendars and entries.
#[derive(Clone)]
pub struct AppState {
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
    /// Wrapped in RwLock for hot-reload support (dev mode pool swapping).
    /// None when SSR is not initialized (e.g., in tests).
    pub ssr_pool: Arc<TokioRwLock<Option<Arc<SsrPool>>>>,
    /// Dev mode reload signal sender (for browser auto-refresh).
    /// Only used when DEV_MODE is set.
    pub dev_reload_tx: broadcast::Sender<()>,
    /// Dev mode build error sender (for browser error overlay).
    /// Only used when DEV_MODE is set.
    pub dev_error_tx: broadcast::Sender<BuildError>,
    /// Dev mode CSS reload sender (for CSS hot-swap without full reload).
    /// Only used when DEV_MODE is set.
    pub dev_css_reload_tx: broadcast::Sender<CssReload>,
}

impl Default for AppState {
    fn default() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        let (dev_reload_tx, _) = broadcast::channel(1);
        let (dev_error_tx, _) = broadcast::channel(1);
        let (dev_css_reload_tx, _) = broadcast::channel(1);
        Self {
            calendars: Arc::new(RwLock::new(HashMap::new())),
            entries: Arc::new(RwLock::new(HashMap::new())),
            event_counter: Arc::new(AtomicU64::new(1)),
            event_history: Arc::new(RwLock::new(VecDeque::new())),
            shutdown_tx,
            ssr_pool: Arc::new(TokioRwLock::new(None)),
            dev_reload_tx,
            dev_error_tx,
            dev_css_reload_tx,
        }
    }
}

impl AppState {
    /// Create a new application state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the SSR pool.
    ///
    /// This is called during initialization before any handlers run,
    /// so there's no contention - use try_write which doesn't block.
    pub fn with_ssr_pool(self, pool: SsrPool) -> Self {
        // At initialization, no contention exists - try_write always succeeds
        let mut guard = self
            .ssr_pool
            .try_write()
            .expect("SSR pool lock should be available during initialization");
        *guard = Some(Arc::new(pool));
        drop(guard);
        self
    }

    /// Get the SSR pool for rendering.
    ///
    /// Returns a clone of the Arc, allowing callers to hold a reference
    /// even if the pool is swapped (hot-reload).
    pub async fn get_ssr_pool(&self) -> Option<Arc<SsrPool>> {
        self.ssr_pool.read().await.clone()
    }

    /// Swap the SSR pool with a new one (dev mode hot-reload).
    ///
    /// The old pool will be dropped, causing workers to terminate.
    pub async fn swap_ssr_pool(&self, new_pool: SsrPool) {
        let mut guard = self.ssr_pool.write().await;
        let old = guard.replace(Arc::new(new_pool));
        drop(guard);

        if old.is_some() {
            tracing::info!("SSR pool swapped (old pool workers will terminate)");
        }
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

    /// Maximum events to keep in history (for reconnection catch-up).
    const MAX_EVENT_HISTORY: usize = 1000;

    /// Publish a calendar event to the event history.
    ///
    /// This stores the event for SSE clients to receive. Events are stored
    /// with a unique incrementing ID for reconnection catch-up support.
    pub fn publish_event(&self, calendar_id: Uuid, event: CalendarEvent) {
        let id = self.event_counter.fetch_add(1, Ordering::SeqCst);
        let stored = StoredEvent {
            id,
            calendar_id,
            event,
        };

        tracing::debug!(event_id = id, %calendar_id, "Publishing calendar event");

        let mut history = self
            .event_history
            .write()
            .expect("Failed to acquire event history write lock");
        history.push_back(stored);

        // Trim old events if history is too large
        while history.len() > Self::MAX_EVENT_HISTORY {
            history.pop_front();
        }
    }

    /// Subscribe to shutdown signal.
    pub fn subscribe_shutdown(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Signal all SSE connections to shut down.
    pub fn signal_shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Subscribe to dev reload signal (for browser auto-refresh).
    pub fn subscribe_dev_reload(&self) -> broadcast::Receiver<()> {
        self.dev_reload_tx.subscribe()
    }

    /// Signal all connected browsers to reload (dev mode only).
    pub fn signal_dev_reload(&self) {
        let _ = self.dev_reload_tx.send(());
        tracing::debug!("Dev reload signal sent");
    }

    /// Subscribe to dev build error signal (for browser error overlay).
    pub fn subscribe_dev_error(&self) -> broadcast::Receiver<BuildError> {
        self.dev_error_tx.subscribe()
    }

    /// Signal all connected browsers to show a build error (dev mode only).
    pub fn signal_dev_error(&self, error: String) {
        let _ = self.dev_error_tx.send(BuildError { error });
        tracing::debug!("Dev error signal sent");
    }

    /// Subscribe to dev CSS reload signal (for CSS hot-swap).
    pub fn subscribe_dev_css_reload(&self) -> broadcast::Receiver<CssReload> {
        self.dev_css_reload_tx.subscribe()
    }

    /// Signal all connected browsers to hot-swap CSS (dev mode only).
    pub fn signal_dev_css_reload(&self, filename: String) {
        let _ = self.dev_css_reload_tx.send(CssReload { filename });
        tracing::debug!("Dev CSS reload signal sent");
    }
}
