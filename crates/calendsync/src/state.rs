use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use uuid::Uuid;

use calendsync_core::calendar::{Calendar, CalendarEntry};

use crate::mock_data::generate_mock_entries;
use crate::models::User;

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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            calendars: Arc::new(RwLock::new(HashMap::new())),
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl AppState {
    /// Create a new application state.
    pub fn new() -> Self {
        Self::default()
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
}
