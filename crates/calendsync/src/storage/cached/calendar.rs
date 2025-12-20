//! Cached calendar repository decorator.
//!
//! Wraps a `CalendarRepository` implementation with cache-aside pattern.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use uuid::Uuid;

use calendsync_core::cache::{
    calendar_entries_pattern, calendar_key, deserialize_calendar, serialize_calendar, Cache,
};
use calendsync_core::calendar::Calendar;
use calendsync_core::storage::{CalendarRepository, Result};

/// Cached calendar repository decorator.
///
/// Implements the cache-aside pattern:
/// - **Reads**: Check cache first, on miss fetch from repository and populate cache
/// - **Writes**: Persist to repository, invalidate cache
///
/// Note: Unlike entries, calendars don't generate SSE events on CRUD operations,
/// so this decorator doesn't use CachePubSub.
///
/// # Type Parameters
///
/// * `R` - The underlying repository implementation
/// * `C` - The cache implementation
pub struct CachedCalendarRepository<R, C>
where
    R: CalendarRepository,
    C: Cache,
{
    repository: Arc<R>,
    cache: Arc<C>,
    ttl: Duration,
}

impl<R, C> CachedCalendarRepository<R, C>
where
    R: CalendarRepository,
    C: Cache,
{
    /// Creates a new cached calendar repository.
    ///
    /// # Arguments
    ///
    /// * `repository` - The underlying repository to cache
    /// * `cache` - The cache implementation
    /// * `ttl` - Time-to-live for cached calendars
    pub fn new(repository: Arc<R>, cache: Arc<C>, ttl: Duration) -> Self {
        Self {
            repository,
            cache,
            ttl,
        }
    }
}

#[async_trait]
impl<R, C> CalendarRepository for CachedCalendarRepository<R, C>
where
    R: CalendarRepository + 'static,
    C: Cache + 'static,
{
    async fn get_calendar(&self, id: Uuid) -> Result<Option<Calendar>> {
        let cache_key = calendar_key(id);

        // Check cache first
        if let Ok(Some(bytes)) = self.cache.get(&cache_key).await {
            if let Ok(calendar) = deserialize_calendar(&bytes) {
                tracing::trace!(calendar_id = %id, "Cache hit for calendar");
                return Ok(Some(calendar));
            }
            // Deserialization failed - treat as cache miss
            tracing::warn!(calendar_id = %id, "Cache calendar deserialization failed");
        }

        // Cache miss - fetch from repository
        tracing::trace!(calendar_id = %id, "Cache miss for calendar");
        let calendar = self.repository.get_calendar(id).await?;

        // Populate cache on hit
        if let Some(ref c) = calendar {
            if let Ok(bytes) = serialize_calendar(c) {
                if let Err(err) = self.cache.set(&cache_key, &bytes, Some(self.ttl)).await {
                    tracing::warn!(calendar_id = %id, error = %err, "Failed to cache calendar");
                }
            }
        }

        Ok(calendar)
    }

    async fn create_calendar(&self, calendar: &Calendar) -> Result<()> {
        // 1. Persist to storage
        self.repository.create_calendar(calendar).await?;

        // 2. Populate cache immediately (cache-aside on write)
        let cache_key = calendar_key(calendar.id);
        if let Ok(bytes) = serialize_calendar(calendar) {
            if let Err(err) = self.cache.set(&cache_key, &bytes, Some(self.ttl)).await {
                tracing::warn!(
                    calendar_id = %calendar.id,
                    error = %err,
                    "Failed to cache new calendar"
                );
            }
        }

        tracing::debug!(calendar_id = %calendar.id, name = %calendar.name, "Calendar created");
        Ok(())
    }

    async fn update_calendar(&self, calendar: &Calendar) -> Result<()> {
        // 1. Persist to storage
        self.repository.update_calendar(calendar).await?;

        // 2. Invalidate cache (will be repopulated on next read)
        let cache_key = calendar_key(calendar.id);
        if let Err(err) = self.cache.delete(&cache_key).await {
            tracing::warn!(
                calendar_id = %calendar.id,
                error = %err,
                "Failed to invalidate calendar cache"
            );
        }

        tracing::debug!(calendar_id = %calendar.id, name = %calendar.name, "Calendar updated");
        Ok(())
    }

    async fn delete_calendar(&self, id: Uuid) -> Result<()> {
        // 1. Persist deletion to storage
        self.repository.delete_calendar(id).await?;

        // 2. Invalidate calendar cache
        let cache_key = calendar_key(id);
        if let Err(err) = self.cache.delete(&cache_key).await {
            tracing::warn!(calendar_id = %id, error = %err, "Failed to invalidate calendar cache");
        }

        // 3. Invalidate all cached entries for this calendar
        // This uses the calendar metadata key deletion behavior which cleans up
        // associated entry caches via set-based tracking
        let pattern = calendar_entries_pattern(id);
        if let Err(err) = self.cache.delete_pattern(&pattern).await {
            tracing::warn!(
                calendar_id = %id,
                error = %err,
                "Failed to invalidate calendar entries cache"
            );
        }

        tracing::debug!(calendar_id = %id, "Calendar deleted");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::RwLock;

    use calendsync_core::cache::Result as CacheResult;

    // Mock repository that tracks calls
    struct MockCalendarRepository {
        calendars: RwLock<HashMap<Uuid, Calendar>>,
        get_calls: AtomicUsize,
    }

    impl MockCalendarRepository {
        fn new() -> Self {
            Self {
                calendars: RwLock::new(HashMap::new()),
                get_calls: AtomicUsize::new(0),
            }
        }

        async fn insert(&self, calendar: Calendar) {
            self.calendars.write().await.insert(calendar.id, calendar);
        }
    }

    #[async_trait]
    impl CalendarRepository for MockCalendarRepository {
        async fn get_calendar(&self, id: Uuid) -> Result<Option<Calendar>> {
            self.get_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.calendars.read().await.get(&id).cloned())
        }

        async fn create_calendar(&self, calendar: &Calendar) -> Result<()> {
            self.calendars
                .write()
                .await
                .insert(calendar.id, calendar.clone());
            Ok(())
        }

        async fn update_calendar(&self, calendar: &Calendar) -> Result<()> {
            self.calendars
                .write()
                .await
                .insert(calendar.id, calendar.clone());
            Ok(())
        }

        async fn delete_calendar(&self, id: Uuid) -> Result<()> {
            self.calendars.write().await.remove(&id);
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

    fn create_test_calendar() -> Calendar {
        Calendar::new("Test Calendar", "#3B82F6")
    }

    #[tokio::test]
    async fn test_get_calendar_cache_miss() {
        let calendar = create_test_calendar();

        let repo = Arc::new(MockCalendarRepository::new());
        repo.insert(calendar.clone()).await;

        let cache = Arc::new(MockCache::new());

        let cached =
            CachedCalendarRepository::new(repo.clone(), cache.clone(), Duration::from_secs(300));

        // First call - should hit repository
        let result = cached.get_calendar(calendar.id).await.unwrap();
        assert_eq!(result.as_ref().map(|c| c.id), Some(calendar.id));
        assert_eq!(repo.get_calls.load(Ordering::SeqCst), 1);

        // Verify cache was populated
        let cache_key = calendar_key(calendar.id);
        assert!(cache.store.read().await.contains_key(&cache_key));
    }

    #[tokio::test]
    async fn test_get_calendar_cache_hit() {
        let calendar = create_test_calendar();

        let repo = Arc::new(MockCalendarRepository::new());
        repo.insert(calendar.clone()).await;

        let cache = Arc::new(MockCache::new());

        let cached =
            CachedCalendarRepository::new(repo.clone(), cache.clone(), Duration::from_secs(300));

        // First call - cache miss
        let _ = cached.get_calendar(calendar.id).await.unwrap();
        assert_eq!(repo.get_calls.load(Ordering::SeqCst), 1);

        // Second call - should hit cache
        let result = cached.get_calendar(calendar.id).await.unwrap();
        assert_eq!(result.as_ref().map(|c| c.id), Some(calendar.id));
        assert_eq!(repo.get_calls.load(Ordering::SeqCst), 1); // Still 1
    }

    #[tokio::test]
    async fn test_create_calendar_populates_cache() {
        let calendar = create_test_calendar();

        let repo = Arc::new(MockCalendarRepository::new());
        let cache = Arc::new(MockCache::new());

        let cached =
            CachedCalendarRepository::new(repo.clone(), cache.clone(), Duration::from_secs(300));

        cached.create_calendar(&calendar).await.unwrap();

        // Cache should be populated
        let cache_key = calendar_key(calendar.id);
        assert!(cache.store.read().await.contains_key(&cache_key));
    }

    #[tokio::test]
    async fn test_update_calendar_invalidates_cache() {
        let calendar = create_test_calendar();

        let repo = Arc::new(MockCalendarRepository::new());
        repo.insert(calendar.clone()).await;

        let cache = Arc::new(MockCache::new());

        let cached =
            CachedCalendarRepository::new(repo.clone(), cache.clone(), Duration::from_secs(300));

        // Populate cache
        let cache_key = calendar_key(calendar.id);
        cache
            .set(&cache_key, b"cached_calendar", None)
            .await
            .unwrap();

        // Update calendar
        cached.update_calendar(&calendar).await.unwrap();

        // Cache should be invalidated
        assert!(!cache.store.read().await.contains_key(&cache_key));
    }

    #[tokio::test]
    async fn test_delete_calendar_invalidates_cache() {
        let calendar = create_test_calendar();

        let repo = Arc::new(MockCalendarRepository::new());
        repo.insert(calendar.clone()).await;

        let cache = Arc::new(MockCache::new());

        let cached =
            CachedCalendarRepository::new(repo.clone(), cache.clone(), Duration::from_secs(300));

        // Populate cache
        let cache_key = calendar_key(calendar.id);
        cache
            .set(&cache_key, b"cached_calendar", None)
            .await
            .unwrap();

        // Delete calendar
        cached.delete_calendar(calendar.id).await.unwrap();

        // Cache should be invalidated
        assert!(!cache.store.read().await.contains_key(&cache_key));
    }
}
