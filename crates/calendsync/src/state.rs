//! Application state with repository-based storage.
//!
//! This module defines the shared application state that is passed to all
//! request handlers. It uses repository trait objects for storage abstraction
//! and supports different backend combinations via feature flags.

use std::{
    collections::{HashSet, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
};
use tokio::sync::{broadcast, RwLock as TokioRwLock};
use uuid::Uuid;

use calendsync_core::cache::CachePubSub;
use calendsync_core::storage::{CalendarRepository, EntryRepository};
use calendsync_ssr::SsrPool;

use crate::config::Config;

// ============================================================================
// Compile-time feature validation
// ============================================================================

// Storage features: exactly one must be enabled, they are mutually exclusive
#[cfg(all(feature = "sqlite", feature = "dynamodb"))]
compile_error!("Cannot enable both 'sqlite' and 'dynamodb' storage features");

#[cfg(all(feature = "sqlite", feature = "inmemory"))]
compile_error!("Cannot enable both 'sqlite' and 'inmemory' storage features");

#[cfg(all(feature = "dynamodb", feature = "inmemory"))]
compile_error!("Cannot enable both 'dynamodb' and 'inmemory' storage features");

#[cfg(not(any(feature = "inmemory", feature = "sqlite", feature = "dynamodb")))]
compile_error!("Must enable exactly one storage feature: 'inmemory', 'sqlite', or 'dynamodb'");

// Cache features: exactly one must be enabled, they are mutually exclusive
#[cfg(all(feature = "memory", feature = "redis"))]
compile_error!("Cannot enable both 'memory' and 'redis' cache features");

#[cfg(not(any(feature = "memory", feature = "redis")))]
compile_error!("Must enable exactly one cache feature: 'memory' or 'redis'");

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
pub use calendsync_core::calendar::CalendarEvent;

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
/// including repository trait objects for database access.
#[derive(Clone)]
pub struct AppState {
    /// Entry repository (cached, wraps underlying storage).
    pub entry_repo: Arc<dyn EntryRepository>,
    /// Calendar repository (cached, wraps underlying storage).
    pub calendar_repo: Arc<dyn CalendarRepository>,
    /// Cache pub/sub for cross-instance event propagation.
    pub cache_pubsub: Arc<dyn CachePubSub>,

    /// Event counter for generating unique event IDs.
    pub event_counter: Arc<AtomicU64>,
    /// Event history for SSE reconnection catch-up.
    pub event_history: Arc<RwLock<VecDeque<StoredEvent>>>,
    /// Maximum events to keep in history.
    event_history_max_size: usize,
    /// Calendars with active event listeners.
    active_listeners: Arc<RwLock<HashSet<Uuid>>>,

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

impl AppState {
    /// Creates a new AppState with the given repositories and configuration.
    fn build(
        entry_repo: Arc<dyn EntryRepository>,
        calendar_repo: Arc<dyn CalendarRepository>,
        cache_pubsub: Arc<dyn CachePubSub>,
        config: &Config,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        let (dev_reload_tx, _) = broadcast::channel(1);
        let (dev_error_tx, _) = broadcast::channel(1);
        let (dev_css_reload_tx, _) = broadcast::channel(1);

        Self {
            entry_repo,
            calendar_repo,
            cache_pubsub,
            event_counter: Arc::new(AtomicU64::new(1)),
            event_history: Arc::new(RwLock::new(VecDeque::new())),
            event_history_max_size: config.event_history_max_size,
            active_listeners: Arc::new(RwLock::new(HashSet::new())),
            shutdown_tx,
            ssr_pool: Arc::new(TokioRwLock::new(None)),
            dev_reload_tx,
            dev_error_tx,
            dev_css_reload_tx,
        }
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

    /// Get the oldest event ID in the history.
    ///
    /// Returns 0 if history is empty.
    pub fn oldest_event_id(&self) -> u64 {
        self.event_history
            .read()
            .ok()
            .and_then(|h| h.front().map(|e| e.id))
            .unwrap_or(0)
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

    /// Store an event in the local event history.
    ///
    /// This is called by the event listener background task when it receives
    /// events from CachePubSub.
    pub fn store_event(&self, calendar_id: Uuid, event: CalendarEvent) {
        let id = self.event_counter.fetch_add(1, Ordering::SeqCst);
        let stored = StoredEvent {
            id,
            calendar_id,
            event,
        };

        tracing::trace!(event_id = id, %calendar_id, "Storing event in history");

        if let Ok(mut history) = self.event_history.write() {
            history.push_back(stored);

            // Trim old events if history is too large
            while history.len() > self.event_history_max_size {
                history.pop_front();
            }
        }
    }

    /// Ensures an event listener is running for the given calendar.
    ///
    /// If a listener is already running, this is a no-op.
    /// Otherwise, spawns a background task that subscribes to CachePubSub
    /// and populates the local event_history.
    pub fn ensure_event_listener(&self, calendar_id: Uuid) {
        // Check if listener already exists
        {
            let listeners = self.active_listeners.read().expect("Lock poisoned");
            if listeners.contains(&calendar_id) {
                return;
            }
        }

        // Register this listener
        {
            let mut listeners = self.active_listeners.write().expect("Lock poisoned");
            // Double-check after acquiring write lock
            if listeners.contains(&calendar_id) {
                return;
            }
            listeners.insert(calendar_id);
        }

        // Spawn the listener task
        let state = self.clone();
        let cache_pubsub = self.cache_pubsub.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let receiver_result = cache_pubsub.subscribe(calendar_id).await;
            let mut receiver = match receiver_result {
                Ok(r) => r,
                Err(err) => {
                    tracing::error!(%calendar_id, error = %err, "Failed to subscribe to calendar events");
                    // Remove from active listeners
                    if let Ok(mut listeners) = state.active_listeners.write() {
                        listeners.remove(&calendar_id);
                    }
                    return;
                }
            };

            tracing::debug!(%calendar_id, "Event listener started");

            loop {
                tokio::select! {
                    result = receiver.recv() => {
                        match result {
                            Ok(event) => {
                                state.store_event(calendar_id, event);
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                tracing::warn!(%calendar_id, lagged = n, "Event listener lagged");
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                tracing::info!(%calendar_id, "Event channel closed");
                                break;
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::debug!(%calendar_id, "Event listener shutting down");
                        break;
                    }
                }
            }

            // Remove from active listeners
            if let Ok(mut listeners) = state.active_listeners.write() {
                listeners.remove(&calendar_id);
            }
        });
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

// ============================================================================
// Factory functions for different backend combinations
// ============================================================================

#[cfg(all(feature = "sqlite", feature = "memory"))]
mod sqlite_memory {
    use super::*;
    use crate::cache::memory::{MemoryCache, MemoryPubSub};
    use crate::storage::cached::{CachedCalendarRepository, CachedEntryRepository};
    use crate::storage::SqliteRepository;

    impl AppState {
        /// Creates AppState with SQLite storage and in-memory cache.
        pub async fn new(config: &Config) -> Result<Self, anyhow::Error> {
            let sqlite_repo = Arc::new(SqliteRepository::new(&config.sqlite_path).await?);
            let memory_cache = Arc::new(MemoryCache::new(config.cache_max_entries));
            let memory_pubsub = Arc::new(MemoryPubSub::new());

            let cached_entry_repo = Arc::new(CachedEntryRepository::new(
                sqlite_repo.clone(),
                memory_cache.clone(),
                memory_pubsub.clone(),
                config.cache_ttl(),
            ));

            let cached_calendar_repo = Arc::new(CachedCalendarRepository::new(
                sqlite_repo,
                memory_cache,
                config.cache_ttl(),
            ));

            Ok(Self::build(
                cached_entry_repo,
                cached_calendar_repo,
                memory_pubsub,
                config,
            ))
        }
    }
}

#[cfg(all(feature = "sqlite", feature = "redis"))]
mod sqlite_redis {
    use super::*;
    use crate::cache::redis_impl::{RedisCache, RedisPubSub};
    use crate::storage::cached::{CachedCalendarRepository, CachedEntryRepository};
    use crate::storage::SqliteRepository;

    impl AppState {
        /// Creates AppState with SQLite storage and Redis cache.
        pub async fn new(config: &Config) -> Result<Self, anyhow::Error> {
            let sqlite_repo = Arc::new(SqliteRepository::new(&config.sqlite_path).await?);
            let redis_cache = Arc::new(RedisCache::new(&config.redis_url).await?);
            let redis_pubsub = Arc::new(RedisPubSub::new(&config.redis_url).await?);

            let cached_entry_repo = Arc::new(CachedEntryRepository::new(
                sqlite_repo.clone(),
                redis_cache.clone(),
                redis_pubsub.clone(),
                config.cache_ttl(),
            ));

            let cached_calendar_repo = Arc::new(CachedCalendarRepository::new(
                sqlite_repo,
                redis_cache,
                config.cache_ttl(),
            ));

            Ok(Self::build(
                cached_entry_repo,
                cached_calendar_repo,
                redis_pubsub,
                config,
            ))
        }
    }
}

#[cfg(all(feature = "inmemory", feature = "memory"))]
mod inmemory_memory {
    use super::*;
    use crate::cache::memory::{MemoryCache, MemoryPubSub};
    use crate::storage::cached::{CachedCalendarRepository, CachedEntryRepository};
    use crate::storage::InMemoryRepository;

    impl AppState {
        /// Creates AppState with in-memory storage and cache.
        /// Useful for testing without any external dependencies.
        pub async fn new(config: &Config) -> Result<Self, anyhow::Error> {
            let inmemory_repo = Arc::new(InMemoryRepository::new());
            let memory_cache = Arc::new(MemoryCache::new(config.cache_max_entries));
            let memory_pubsub = Arc::new(MemoryPubSub::new());

            let cached_entry_repo = Arc::new(CachedEntryRepository::new(
                inmemory_repo.clone(),
                memory_cache.clone(),
                memory_pubsub.clone(),
                config.cache_ttl(),
            ));

            let cached_calendar_repo = Arc::new(CachedCalendarRepository::new(
                inmemory_repo,
                memory_cache,
                config.cache_ttl(),
            ));

            Ok(Self::build(
                cached_entry_repo,
                cached_calendar_repo,
                memory_pubsub,
                config,
            ))
        }
    }
}

#[cfg(all(feature = "dynamodb", feature = "memory"))]
mod dynamodb_memory {
    use super::*;
    use crate::cache::memory::{MemoryCache, MemoryPubSub};
    use crate::storage::cached::{CachedCalendarRepository, CachedEntryRepository};
    use crate::storage::DynamoDbRepository;

    impl AppState {
        /// Creates AppState with DynamoDB storage and in-memory cache.
        pub async fn new(config: &Config) -> Result<Self, anyhow::Error> {
            let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let dynamodb_client = aws_sdk_dynamodb::Client::new(&aws_config);
            let dynamodb_repo = Arc::new(DynamoDbRepository::new(
                dynamodb_client,
                "calendsync".to_string(),
            ));

            let memory_cache = Arc::new(MemoryCache::new(config.cache_max_entries));
            let memory_pubsub = Arc::new(MemoryPubSub::new());

            let cached_entry_repo = Arc::new(CachedEntryRepository::new(
                dynamodb_repo.clone(),
                memory_cache.clone(),
                memory_pubsub.clone(),
                config.cache_ttl(),
            ));

            let cached_calendar_repo = Arc::new(CachedCalendarRepository::new(
                dynamodb_repo,
                memory_cache,
                config.cache_ttl(),
            ));

            Ok(Self::build(
                cached_entry_repo,
                cached_calendar_repo,
                memory_pubsub,
                config,
            ))
        }
    }
}

#[cfg(all(feature = "dynamodb", feature = "redis"))]
mod dynamodb_redis {
    use super::*;
    use crate::cache::redis_impl::{RedisCache, RedisPubSub};
    use crate::storage::cached::{CachedCalendarRepository, CachedEntryRepository};
    use crate::storage::DynamoDbRepository;

    impl AppState {
        /// Creates AppState with DynamoDB storage and Redis cache.
        pub async fn new(config: &Config) -> Result<Self, anyhow::Error> {
            let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let dynamodb_client = aws_sdk_dynamodb::Client::new(&aws_config);
            let dynamodb_repo = Arc::new(DynamoDbRepository::new(
                dynamodb_client,
                "calendsync".to_string(),
            ));

            let redis_cache = Arc::new(RedisCache::new(&config.redis_url).await?);
            let redis_pubsub = Arc::new(RedisPubSub::new(&config.redis_url).await?);

            let cached_entry_repo = Arc::new(CachedEntryRepository::new(
                dynamodb_repo.clone(),
                redis_cache.clone(),
                redis_pubsub.clone(),
                config.cache_ttl(),
            ));

            let cached_calendar_repo = Arc::new(CachedCalendarRepository::new(
                dynamodb_repo,
                redis_cache,
                config.cache_ttl(),
            ));

            Ok(Self::build(
                cached_entry_repo,
                cached_calendar_repo,
                redis_pubsub,
                config,
            ))
        }
    }
}

// ============================================================================
// Test support - provides Default implementation for unit tests
// ============================================================================

#[cfg(test)]
mod test_support {
    use super::*;
    use crate::cache::memory::MemoryPubSub;

    use std::collections::HashMap;

    use async_trait::async_trait;
    use tokio::sync::RwLock;

    use calendsync_core::calendar::{Calendar, CalendarEntry};
    use calendsync_core::storage::{CalendarRepository, DateRange, EntryRepository, Result};

    /// Minimal in-memory repository for tests.
    /// This is a simplified version that only implements the traits needed for testing.
    #[derive(Debug, Default)]
    struct TestRepository {
        entries: RwLock<HashMap<Uuid, CalendarEntry>>,
        calendars: RwLock<HashMap<Uuid, Calendar>>,
    }

    #[async_trait]
    impl EntryRepository for TestRepository {
        async fn get_entry(&self, id: Uuid) -> Result<Option<CalendarEntry>> {
            let entries = self.entries.read().await;
            Ok(entries.get(&id).cloned())
        }

        async fn get_entries_by_calendar(
            &self,
            calendar_id: Uuid,
            date_range: DateRange,
        ) -> Result<Vec<CalendarEntry>> {
            let entries = self.entries.read().await;
            let filtered: Vec<CalendarEntry> = entries
                .values()
                .filter(|entry| {
                    entry.calendar_id == calendar_id
                        && entry.start_date <= date_range.end
                        && entry.end_date >= date_range.start
                })
                .cloned()
                .collect();
            Ok(filtered)
        }

        async fn create_entry(&self, entry: &CalendarEntry) -> Result<()> {
            let mut entries = self.entries.write().await;
            entries.insert(entry.id, entry.clone());
            Ok(())
        }

        async fn update_entry(&self, entry: &CalendarEntry) -> Result<()> {
            let mut entries = self.entries.write().await;
            entries.insert(entry.id, entry.clone());
            Ok(())
        }

        async fn delete_entry(&self, id: Uuid) -> Result<()> {
            let mut entries = self.entries.write().await;
            entries.remove(&id);
            Ok(())
        }
    }

    #[async_trait]
    impl CalendarRepository for TestRepository {
        async fn get_calendar(&self, id: Uuid) -> Result<Option<Calendar>> {
            let calendars = self.calendars.read().await;
            Ok(calendars.get(&id).cloned())
        }

        async fn create_calendar(&self, calendar: &Calendar) -> Result<()> {
            let mut calendars = self.calendars.write().await;
            calendars.insert(calendar.id, calendar.clone());
            Ok(())
        }

        async fn update_calendar(&self, calendar: &Calendar) -> Result<()> {
            let mut calendars = self.calendars.write().await;
            calendars.insert(calendar.id, calendar.clone());
            Ok(())
        }

        async fn delete_calendar(&self, id: Uuid) -> Result<()> {
            let mut calendars = self.calendars.write().await;
            calendars.remove(&id);
            Ok(())
        }
    }

    impl Default for AppState {
        /// Creates an AppState with in-memory storage for testing.
        ///
        /// This is only available in test builds and provides a simple way
        /// to create an AppState without external dependencies.
        fn default() -> Self {
            let config = Config::default();
            let test_repo = Arc::new(TestRepository::default());
            let memory_pubsub = Arc::new(MemoryPubSub::new());

            // For tests, we use the test repository without caching
            Self::build(test_repo.clone(), test_repo, memory_pubsub, &config)
        }
    }
}
