//! Cached entry repository decorator.
//!
//! Wraps an `EntryRepository` implementation with cache-aside pattern and
//! event publishing for real-time updates.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use uuid::Uuid;

use calendsync_core::cache::{
    calendar_entries_key, calendar_entries_pattern, deserialize_entries, deserialize_entry,
    entry_key, serialize_entries, serialize_entry, Cache, CachePubSub,
};
use calendsync_core::calendar::{CalendarEntry, CalendarEvent};
use calendsync_core::storage::{DateRange, EntryRepository, Result};

/// Cached entry repository decorator.
///
/// Implements the cache-aside pattern:
/// - **Reads**: Check cache first, on miss fetch from repository and populate cache
/// - **Writes**: Persist to repository, invalidate cache, publish events via pubsub
///
/// # Type Parameters
///
/// * `R` - The underlying repository implementation
/// * `C` - The cache implementation
/// * `P` - The pub/sub implementation for cross-instance event propagation
pub struct CachedEntryRepository<R, C, P>
where
    R: EntryRepository,
    C: Cache,
    P: CachePubSub,
{
    repository: Arc<R>,
    cache: Arc<C>,
    pubsub: Arc<P>,
    ttl: Duration,
}

impl<R, C, P> CachedEntryRepository<R, C, P>
where
    R: EntryRepository,
    C: Cache,
    P: CachePubSub,
{
    /// Creates a new cached entry repository.
    ///
    /// # Arguments
    ///
    /// * `repository` - The underlying repository to cache
    /// * `cache` - The cache implementation
    /// * `pubsub` - The pub/sub implementation for event propagation
    /// * `ttl` - Time-to-live for cached entries
    pub fn new(repository: Arc<R>, cache: Arc<C>, pubsub: Arc<P>, ttl: Duration) -> Self {
        Self {
            repository,
            cache,
            pubsub,
            ttl,
        }
    }
}

#[async_trait]
impl<R, C, P> EntryRepository for CachedEntryRepository<R, C, P>
where
    R: EntryRepository + 'static,
    C: Cache + 'static,
    P: CachePubSub + 'static,
{
    async fn get_entry(&self, id: Uuid) -> Result<Option<CalendarEntry>> {
        let cache_key = entry_key(id);

        // Check cache first
        if let Ok(Some(bytes)) = self.cache.get(&cache_key).await {
            if let Ok(entry) = deserialize_entry(&bytes) {
                tracing::trace!(entry_id = %id, "Cache hit for entry");
                return Ok(Some(entry));
            }
            // Deserialization failed - treat as cache miss
            tracing::warn!(entry_id = %id, "Cache entry deserialization failed");
        }

        // Cache miss - fetch from repository
        tracing::trace!(entry_id = %id, "Cache miss for entry");
        let entry = self.repository.get_entry(id).await?;

        // Populate cache on hit
        if let Some(ref e) = entry {
            if let Ok(bytes) = serialize_entry(e) {
                if let Err(err) = self.cache.set(&cache_key, &bytes, Some(self.ttl)).await {
                    tracing::warn!(entry_id = %id, error = %err, "Failed to cache entry");
                }
            }
        }

        Ok(entry)
    }

    async fn get_entries_by_calendar(
        &self,
        calendar_id: Uuid,
        date_range: DateRange,
    ) -> Result<Vec<CalendarEntry>> {
        let cache_key = calendar_entries_key(calendar_id, date_range.start, date_range.end);

        // Check cache first
        if let Ok(Some(bytes)) = self.cache.get(&cache_key).await {
            if let Ok(entries) = deserialize_entries(&bytes) {
                tracing::trace!(
                    %calendar_id,
                    start = %date_range.start,
                    end = %date_range.end,
                    count = entries.len(),
                    "Cache hit for calendar entries"
                );
                return Ok(entries);
            }
            // Deserialization failed - treat as cache miss
            tracing::warn!(%calendar_id, "Cache entries deserialization failed");
        }

        // Cache miss - fetch from repository
        tracing::trace!(
            %calendar_id,
            start = %date_range.start,
            end = %date_range.end,
            "Cache miss for calendar entries"
        );
        let entries = self
            .repository
            .get_entries_by_calendar(calendar_id, date_range)
            .await?;

        // Populate cache
        if let Ok(bytes) = serialize_entries(&entries) {
            if let Err(err) = self.cache.set(&cache_key, &bytes, Some(self.ttl)).await {
                tracing::warn!(%calendar_id, error = %err, "Failed to cache calendar entries");
            }
        }

        Ok(entries)
    }

    async fn create_entry(&self, entry: &CalendarEntry) -> Result<()> {
        // 1. Persist to storage
        self.repository.create_entry(entry).await?;

        // 2. Invalidate calendar entries cache (all date ranges for this calendar)
        let pattern = calendar_entries_pattern(entry.calendar_id);
        if let Err(err) = self.cache.delete_pattern(&pattern).await {
            tracing::warn!(
                calendar_id = %entry.calendar_id,
                error = %err,
                "Failed to invalidate calendar entries cache"
            );
        }

        // 3. Publish event for cross-instance propagation
        let event = CalendarEvent::entry_added(entry.clone());
        if let Err(err) = self.pubsub.publish(entry.calendar_id, &event).await {
            tracing::warn!(
                calendar_id = %entry.calendar_id,
                entry_id = %entry.id,
                error = %err,
                "Failed to publish entry_added event"
            );
        }

        tracing::debug!(entry_id = %entry.id, calendar_id = %entry.calendar_id, "Entry created");
        Ok(())
    }

    async fn update_entry(&self, entry: &CalendarEntry) -> Result<()> {
        // 1. Persist to storage
        self.repository.update_entry(entry).await?;

        // 2. Invalidate specific entry cache
        let entry_cache_key = entry_key(entry.id);
        if let Err(err) = self.cache.delete(&entry_cache_key).await {
            tracing::warn!(
                entry_id = %entry.id,
                error = %err,
                "Failed to invalidate entry cache"
            );
        }

        // 3. Invalidate calendar entries cache (all date ranges)
        let pattern = calendar_entries_pattern(entry.calendar_id);
        if let Err(err) = self.cache.delete_pattern(&pattern).await {
            tracing::warn!(
                calendar_id = %entry.calendar_id,
                error = %err,
                "Failed to invalidate calendar entries cache"
            );
        }

        // 4. Publish event for cross-instance propagation
        let event = CalendarEvent::entry_updated(entry.clone());
        if let Err(err) = self.pubsub.publish(entry.calendar_id, &event).await {
            tracing::warn!(
                calendar_id = %entry.calendar_id,
                entry_id = %entry.id,
                error = %err,
                "Failed to publish entry_updated event"
            );
        }

        tracing::debug!(entry_id = %entry.id, calendar_id = %entry.calendar_id, "Entry updated");
        Ok(())
    }

    async fn delete_entry(&self, id: Uuid) -> Result<()> {
        // Get entry first for event data (need calendar_id and date)
        let entry = self.repository.get_entry(id).await?;

        // 1. Persist deletion to storage
        self.repository.delete_entry(id).await?;

        // 2. Invalidate specific entry cache
        let entry_cache_key = entry_key(id);
        if let Err(err) = self.cache.delete(&entry_cache_key).await {
            tracing::warn!(entry_id = %id, error = %err, "Failed to invalidate entry cache");
        }

        // 3. Invalidate calendar entries cache and publish event (if we had entry data)
        if let Some(e) = entry {
            let pattern = calendar_entries_pattern(e.calendar_id);
            if let Err(err) = self.cache.delete_pattern(&pattern).await {
                tracing::warn!(
                    calendar_id = %e.calendar_id,
                    error = %err,
                    "Failed to invalidate calendar entries cache"
                );
            }

            // 4. Publish event for cross-instance propagation
            let event = CalendarEvent::entry_deleted(e.id, e.date);
            if let Err(err) = self.pubsub.publish(e.calendar_id, &event).await {
                tracing::warn!(
                    calendar_id = %e.calendar_id,
                    entry_id = %id,
                    error = %err,
                    "Failed to publish entry_deleted event"
                );
            }

            tracing::debug!(entry_id = %id, calendar_id = %e.calendar_id, "Entry deleted");
        } else {
            tracing::debug!(entry_id = %id, "Entry deleted (no calendar context for event)");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::{broadcast, RwLock};

    use calendsync_core::cache::Result as CacheResult;

    // Mock repository that tracks calls
    struct MockEntryRepository {
        entries: RwLock<HashMap<Uuid, CalendarEntry>>,
        get_entry_calls: AtomicUsize,
        get_entries_calls: AtomicUsize,
    }

    impl MockEntryRepository {
        fn new() -> Self {
            Self {
                entries: RwLock::new(HashMap::new()),
                get_entry_calls: AtomicUsize::new(0),
                get_entries_calls: AtomicUsize::new(0),
            }
        }

        async fn insert(&self, entry: CalendarEntry) {
            self.entries.write().await.insert(entry.id, entry);
        }
    }

    #[async_trait]
    impl EntryRepository for MockEntryRepository {
        async fn get_entry(&self, id: Uuid) -> Result<Option<CalendarEntry>> {
            self.get_entry_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.entries.read().await.get(&id).cloned())
        }

        async fn get_entries_by_calendar(
            &self,
            calendar_id: Uuid,
            date_range: DateRange,
        ) -> Result<Vec<CalendarEntry>> {
            self.get_entries_calls.fetch_add(1, Ordering::SeqCst);
            let entries = self.entries.read().await;
            Ok(entries
                .values()
                .filter(|e| {
                    e.calendar_id == calendar_id
                        && e.date >= date_range.start
                        && e.date <= date_range.end
                })
                .cloned()
                .collect())
        }

        async fn create_entry(&self, entry: &CalendarEntry) -> Result<()> {
            self.entries.write().await.insert(entry.id, entry.clone());
            Ok(())
        }

        async fn update_entry(&self, entry: &CalendarEntry) -> Result<()> {
            self.entries.write().await.insert(entry.id, entry.clone());
            Ok(())
        }

        async fn delete_entry(&self, id: Uuid) -> Result<()> {
            self.entries.write().await.remove(&id);
            Ok(())
        }
    }

    // Mock cache
    struct MockCache {
        store: RwLock<HashMap<String, Vec<u8>>>,
    }

    impl MockCache {
        fn new() -> Self {
            Self {
                store: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl Cache for MockCache {
        async fn get(&self, key: &str) -> CacheResult<Option<Vec<u8>>> {
            Ok(self.store.read().await.get(key).cloned())
        }

        async fn set(&self, key: &str, value: &[u8], _ttl: Option<Duration>) -> CacheResult<()> {
            self.store
                .write()
                .await
                .insert(key.to_string(), value.to_vec());
            Ok(())
        }

        async fn delete(&self, key: &str) -> CacheResult<()> {
            self.store.write().await.remove(key);
            Ok(())
        }

        async fn delete_pattern(&self, pattern: &str) -> CacheResult<()> {
            let mut store = self.store.write().await;
            let keys: Vec<_> = store
                .keys()
                .filter(|k| calendsync_core::cache::pattern_matches(pattern, k))
                .cloned()
                .collect();
            for key in keys {
                store.remove(&key);
            }
            Ok(())
        }
    }

    // Mock pubsub
    struct MockPubSub {
        events: RwLock<Vec<(Uuid, CalendarEvent)>>,
        sender: broadcast::Sender<CalendarEvent>,
    }

    impl MockPubSub {
        fn new() -> Self {
            let (sender, _) = broadcast::channel(100);
            Self {
                events: RwLock::new(Vec::new()),
                sender,
            }
        }

        async fn published_events(&self) -> Vec<(Uuid, CalendarEvent)> {
            self.events.read().await.clone()
        }
    }

    #[async_trait]
    impl CachePubSub for MockPubSub {
        async fn publish(&self, calendar_id: Uuid, event: &CalendarEvent) -> CacheResult<()> {
            self.events.write().await.push((calendar_id, event.clone()));
            let _ = self.sender.send(event.clone());
            Ok(())
        }

        async fn subscribe(
            &self,
            _calendar_id: Uuid,
        ) -> CacheResult<broadcast::Receiver<CalendarEvent>> {
            Ok(self.sender.subscribe())
        }
    }

    fn create_test_entry(calendar_id: Uuid, date: NaiveDate) -> CalendarEntry {
        CalendarEntry::all_day(calendar_id, "Test Event", date)
    }

    #[tokio::test]
    async fn test_get_entry_cache_miss_fetches_from_repo() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = create_test_entry(calendar_id, date);

        let repo = Arc::new(MockEntryRepository::new());
        repo.insert(entry.clone()).await;

        let cache = Arc::new(MockCache::new());
        let pubsub = Arc::new(MockPubSub::new());

        let cached = CachedEntryRepository::new(
            repo.clone(),
            cache.clone(),
            pubsub,
            Duration::from_secs(300),
        );

        // First call - should hit repository
        let result = cached.get_entry(entry.id).await.unwrap();
        assert_eq!(result.as_ref().map(|e| e.id), Some(entry.id));
        assert_eq!(repo.get_entry_calls.load(Ordering::SeqCst), 1);

        // Verify cache was populated
        let cache_key = entry_key(entry.id);
        assert!(cache.store.read().await.contains_key(&cache_key));
    }

    #[tokio::test]
    async fn test_get_entry_cache_hit_returns_cached() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = create_test_entry(calendar_id, date);

        let repo = Arc::new(MockEntryRepository::new());
        repo.insert(entry.clone()).await;

        let cache = Arc::new(MockCache::new());
        let pubsub = Arc::new(MockPubSub::new());

        let cached = CachedEntryRepository::new(
            repo.clone(),
            cache.clone(),
            pubsub,
            Duration::from_secs(300),
        );

        // First call - cache miss
        let _ = cached.get_entry(entry.id).await.unwrap();
        assert_eq!(repo.get_entry_calls.load(Ordering::SeqCst), 1);

        // Second call - should hit cache
        let result = cached.get_entry(entry.id).await.unwrap();
        assert_eq!(result.as_ref().map(|e| e.id), Some(entry.id));
        assert_eq!(repo.get_entry_calls.load(Ordering::SeqCst), 1); // Still 1
    }

    #[tokio::test]
    async fn test_get_entries_by_calendar_cache_miss() {
        let calendar_id = Uuid::new_v4();
        let date1 = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2024, 6, 20).unwrap();

        let repo = Arc::new(MockEntryRepository::new());
        repo.insert(create_test_entry(calendar_id, date1)).await;
        repo.insert(create_test_entry(calendar_id, date2)).await;

        let cache = Arc::new(MockCache::new());
        let pubsub = Arc::new(MockPubSub::new());

        let cached = CachedEntryRepository::new(
            repo.clone(),
            cache.clone(),
            pubsub,
            Duration::from_secs(300),
        );

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
        )
        .unwrap();

        let entries = cached
            .get_entries_by_calendar(calendar_id, range)
            .await
            .unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(repo.get_entries_calls.load(Ordering::SeqCst), 1);

        // Verify cache was populated
        let cache_key = calendar_entries_key(calendar_id, range.start, range.end);
        assert!(cache.store.read().await.contains_key(&cache_key));
    }

    #[tokio::test]
    async fn test_get_entries_by_calendar_cache_hit() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        let repo = Arc::new(MockEntryRepository::new());
        repo.insert(create_test_entry(calendar_id, date)).await;

        let cache = Arc::new(MockCache::new());
        let pubsub = Arc::new(MockPubSub::new());

        let cached = CachedEntryRepository::new(
            repo.clone(),
            cache.clone(),
            pubsub,
            Duration::from_secs(300),
        );

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
        )
        .unwrap();

        // First call
        let _ = cached
            .get_entries_by_calendar(calendar_id, range)
            .await
            .unwrap();
        assert_eq!(repo.get_entries_calls.load(Ordering::SeqCst), 1);

        // Second call - should hit cache
        let entries = cached
            .get_entries_by_calendar(calendar_id, range)
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(repo.get_entries_calls.load(Ordering::SeqCst), 1); // Still 1
    }

    #[tokio::test]
    async fn test_create_entry_invalidates_pattern() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        let repo = Arc::new(MockEntryRepository::new());
        let cache = Arc::new(MockCache::new());
        let pubsub = Arc::new(MockPubSub::new());

        let cached = CachedEntryRepository::new(
            repo.clone(),
            cache.clone(),
            pubsub.clone(),
            Duration::from_secs(300),
        );

        // Pre-populate cache with entries for this calendar
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
        )
        .unwrap();
        let cache_key = calendar_entries_key(calendar_id, range.start, range.end);
        cache
            .set(&cache_key, b"cached_entries", None)
            .await
            .unwrap();

        // Create new entry
        let entry = create_test_entry(calendar_id, date);
        cached.create_entry(&entry).await.unwrap();

        // Cache should be invalidated
        assert!(!cache.store.read().await.contains_key(&cache_key));
    }

    #[tokio::test]
    async fn test_create_entry_publishes_event() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        let repo = Arc::new(MockEntryRepository::new());
        let cache = Arc::new(MockCache::new());
        let pubsub = Arc::new(MockPubSub::new());

        let cached = CachedEntryRepository::new(
            repo.clone(),
            cache.clone(),
            pubsub.clone(),
            Duration::from_secs(300),
        );

        let entry = create_test_entry(calendar_id, date);
        cached.create_entry(&entry).await.unwrap();

        let events = pubsub.published_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, calendar_id);
        assert!(matches!(events[0].1, CalendarEvent::EntryAdded { .. }));
    }

    #[tokio::test]
    async fn test_update_entry_invalidates_entry_and_pattern() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = create_test_entry(calendar_id, date);

        let repo = Arc::new(MockEntryRepository::new());
        repo.insert(entry.clone()).await;

        let cache = Arc::new(MockCache::new());
        let pubsub = Arc::new(MockPubSub::new());

        let cached = CachedEntryRepository::new(
            repo.clone(),
            cache.clone(),
            pubsub.clone(),
            Duration::from_secs(300),
        );

        // Pre-populate cache
        let entry_cache_key = entry_key(entry.id);
        cache
            .set(&entry_cache_key, b"cached_entry", None)
            .await
            .unwrap();

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
        )
        .unwrap();
        let entries_cache_key = calendar_entries_key(calendar_id, range.start, range.end);
        cache
            .set(&entries_cache_key, b"cached_entries", None)
            .await
            .unwrap();

        // Update entry
        cached.update_entry(&entry).await.unwrap();

        // Both caches should be invalidated
        assert!(!cache.store.read().await.contains_key(&entry_cache_key));
        assert!(!cache.store.read().await.contains_key(&entries_cache_key));
    }

    #[tokio::test]
    async fn test_update_entry_publishes_event() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = create_test_entry(calendar_id, date);

        let repo = Arc::new(MockEntryRepository::new());
        repo.insert(entry.clone()).await;

        let cache = Arc::new(MockCache::new());
        let pubsub = Arc::new(MockPubSub::new());

        let cached = CachedEntryRepository::new(
            repo.clone(),
            cache.clone(),
            pubsub.clone(),
            Duration::from_secs(300),
        );

        cached.update_entry(&entry).await.unwrap();

        let events = pubsub.published_events().await;
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].1, CalendarEvent::EntryUpdated { .. }));
    }

    #[tokio::test]
    async fn test_delete_entry_invalidates_and_publishes() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = create_test_entry(calendar_id, date);

        let repo = Arc::new(MockEntryRepository::new());
        repo.insert(entry.clone()).await;

        let cache = Arc::new(MockCache::new());
        let pubsub = Arc::new(MockPubSub::new());

        let cached = CachedEntryRepository::new(
            repo.clone(),
            cache.clone(),
            pubsub.clone(),
            Duration::from_secs(300),
        );

        // Pre-populate cache
        let entry_cache_key = entry_key(entry.id);
        cache
            .set(&entry_cache_key, b"cached_entry", None)
            .await
            .unwrap();

        // Delete entry
        cached.delete_entry(entry.id).await.unwrap();

        // Cache should be invalidated
        assert!(!cache.store.read().await.contains_key(&entry_cache_key));

        // Event should be published
        let events = pubsub.published_events().await;
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].1, CalendarEvent::EntryDeleted { .. }));
    }
}
