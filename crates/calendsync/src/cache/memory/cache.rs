//! In-memory cache implementation with LRU eviction.
//!
//! Provides a thread-safe in-memory cache with TTL support using
//! tokio synchronization primitives and LRU eviction policy.
//!
//! This implementation mirrors the Redis cache behavior for consistency:
//! - Calendar entry keys are tracked per calendar for efficient pattern deletion
//! - Deleting a calendar metadata key (`calendar:{id}`) cleans up all associated entries
//! - Deleting a calendar entry key removes it from tracking

use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use lru::LruCache;
use tokio::sync::RwLock;
use uuid::Uuid;

use calendsync_core::cache::{
    extract_calendar_id_from_key, extract_calendar_id_from_pattern, is_calendar_entry_key,
    is_calendar_metadata_key, pattern_matches, Cache, Result,
};

/// A single cache entry with optional expiration.
#[derive(Debug, Clone)]
struct CacheEntry {
    value: Vec<u8>,
    expires_at: Option<Instant>,
}

impl CacheEntry {
    /// Creates a new cache entry with optional TTL.
    fn new(value: Vec<u8>, ttl: Option<Duration>) -> Self {
        let expires_at = ttl.map(|d| Instant::now() + d);
        Self { value, expires_at }
    }

    /// Returns true if this entry has expired.
    fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| Instant::now() > exp)
    }
}

/// In-memory cache implementation with LRU eviction.
///
/// Thread-safe cache using `Arc<RwLock<LruCache>>` for concurrent access.
/// Supports TTL with lazy expiration (entries are cleaned up on access).
/// Uses LRU eviction to limit memory usage when max_entries is reached.
///
/// Calendar entry keys are tracked per calendar ID to enable efficient
/// pattern deletion and full cleanup when a calendar is deleted.
#[derive(Debug, Clone)]
pub struct MemoryCache {
    /// Main key-value store with LRU eviction.
    store: Arc<RwLock<LruCache<String, CacheEntry>>>,
    /// Tracks calendar entry keys by calendar ID for efficient cleanup.
    /// Maps calendar_id -> Set of cache keys.
    tracking: Arc<RwLock<HashMap<Uuid, HashSet<String>>>>,
}

impl MemoryCache {
    /// Creates a new in-memory cache with LRU eviction.
    ///
    /// # Arguments
    ///
    /// * `max_entries` - Maximum number of entries before LRU eviction kicks in.
    ///
    /// # Panics
    ///
    /// Panics if `max_entries` is 0.
    pub fn new(max_entries: usize) -> Self {
        let capacity = NonZeroUsize::new(max_entries).expect("max_entries must be > 0");
        Self {
            store: Arc::new(RwLock::new(LruCache::new(capacity))),
            tracking: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Cache for MemoryCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut store = self.store.write().await;

        match store.get(key) {
            Some(entry) if entry.is_expired() => {
                // Entry exists but is expired - return None
                // Note: We do lazy cleanup, so we don't remove it here.
                // A production implementation might want to spawn a cleanup task.
                Ok(None)
            }
            Some(entry) => Ok(Some(entry.value.clone())),
            None => Ok(None),
        }
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<()> {
        // Store the value
        {
            let mut store = self.store.write().await;
            let entry = CacheEntry::new(value.to_vec(), ttl);
            store.put(key.to_string(), entry);
        }

        // Track calendar entry keys for efficient cleanup
        if is_calendar_entry_key(key) {
            if let Some(calendar_id) = extract_calendar_id_from_key(key) {
                let mut tracking = self.tracking.write().await;
                tracking
                    .entry(calendar_id)
                    .or_default()
                    .insert(key.to_string());
            }
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        // Case 1: Deleting a calendar metadata key - full cleanup
        if is_calendar_metadata_key(key) {
            if let Some(calendar_id) = extract_calendar_id_from_key(key) {
                // Get all tracked keys for this calendar
                let tracked_keys = {
                    let mut tracking = self.tracking.write().await;
                    tracking.remove(&calendar_id).unwrap_or_default()
                };

                // Delete all tracked keys
                if !tracked_keys.is_empty() {
                    let mut store = self.store.write().await;
                    for tracked_key in &tracked_keys {
                        store.pop(tracked_key);
                    }
                }
            }
        }
        // Case 2: Deleting a calendar entry key - remove from tracking
        else if is_calendar_entry_key(key) {
            if let Some(calendar_id) = extract_calendar_id_from_key(key) {
                let mut tracking = self.tracking.write().await;
                if let Some(keys) = tracking.get_mut(&calendar_id) {
                    keys.remove(key);
                    // Clean up empty tracking sets
                    if keys.is_empty() {
                        tracking.remove(&calendar_id);
                    }
                }
            }
        }
        // Case 3: Non-calendar key - just delete (no tracking involved)

        // Delete the key itself
        let mut store = self.store.write().await;
        store.pop(key);

        Ok(())
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        // Extract calendar ID from pattern for efficient lookup
        let Some(calendar_id) = extract_calendar_id_from_pattern(pattern) else {
            // Non-calendar pattern - fall back to full iteration
            // This is O(n) but only for non-calendar patterns
            let mut store = self.store.write().await;
            let keys_to_delete: Vec<String> = store
                .iter()
                .filter(|(key, _)| pattern_matches(pattern, key))
                .map(|(key, _)| key.clone())
                .collect();
            for key in keys_to_delete {
                store.pop(&key);
            }
            return Ok(());
        };

        // Get tracked keys for this calendar
        let tracked_keys: Vec<String> = {
            let tracking = self.tracking.read().await;
            tracking
                .get(&calendar_id)
                .map(|keys| keys.iter().cloned().collect())
                .unwrap_or_default()
        };

        // Filter keys that match the pattern
        let keys_to_delete: Vec<String> = tracked_keys
            .into_iter()
            .filter(|k| pattern_matches(pattern, k))
            .collect();

        if !keys_to_delete.is_empty() {
            // Delete matching keys from store
            {
                let mut store = self.store.write().await;
                for key in &keys_to_delete {
                    store.pop(key);
                }
            }

            // Remove from tracking
            {
                let mut tracking = self.tracking.write().await;
                if let Some(keys) = tracking.get_mut(&calendar_id) {
                    for key in &keys_to_delete {
                        keys.remove(key);
                    }
                    // Clean up empty tracking sets
                    if keys.is_empty() {
                        tracking.remove(&calendar_id);
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use calendsync_core::cache::{calendar_entries_key, calendar_key};
    use chrono::NaiveDate;
    use std::time::Duration;

    /// Default max entries for tests
    const TEST_MAX_ENTRIES: usize = 1000;

    #[tokio::test]
    async fn test_set_and_get() {
        let cache = MemoryCache::new(TEST_MAX_ENTRIES);
        let key = "test:key";
        let value = b"test value";

        cache.set(key, value, None).await.unwrap();
        let result = cache.get(key).await.unwrap();

        assert_eq!(result, Some(value.to_vec()));
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let cache = MemoryCache::new(TEST_MAX_ENTRIES);
        let result = cache.get("nonexistent:key").await.unwrap();

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_delete() {
        let cache = MemoryCache::new(TEST_MAX_ENTRIES);
        let key = "test:delete";
        let value = b"to be deleted";

        cache.set(key, value, None).await.unwrap();
        assert!(cache.get(key).await.unwrap().is_some());

        cache.delete(key).await.unwrap();
        assert!(cache.get(key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let cache = MemoryCache::new(TEST_MAX_ENTRIES);
        let key = "test:ttl";
        let value = b"short-lived";

        // Set with a very short TTL
        cache
            .set(key, value, Some(Duration::from_millis(50)))
            .await
            .unwrap();

        // Should exist immediately
        assert!(cache.get(key).await.unwrap().is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be expired now
        assert!(cache.get(key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_pattern() {
        let cache = MemoryCache::new(TEST_MAX_ENTRIES);

        // Use proper calendar-formatted keys so they get tracked
        let calendar_id = Uuid::new_v4();
        let start1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end1 = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let start2 = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
        let end2 = NaiveDate::from_ymd_opt(2024, 2, 28).unwrap();

        let key1 = calendar_entries_key(calendar_id, start1, end1);
        let key2 = calendar_entries_key(calendar_id, start2, end2);

        // Another calendar's entries
        let other_calendar_id = Uuid::new_v4();
        let key3 = calendar_entries_key(other_calendar_id, start1, end1);

        cache.set(&key1, b"1", None).await.unwrap();
        cache.set(&key2, b"2", None).await.unwrap();
        cache.set(&key3, b"3", None).await.unwrap();
        cache.set("user:123", b"4", None).await.unwrap();

        // Delete pattern for first calendar entries
        let pattern = format!("calendar:{}:entries:*", calendar_id);
        cache.delete_pattern(&pattern).await.unwrap();

        // First calendar entries should be gone
        assert!(cache.get(&key1).await.unwrap().is_none());
        assert!(cache.get(&key2).await.unwrap().is_none());

        // Other entries should remain
        assert!(cache.get(&key3).await.unwrap().is_some());
        assert!(cache.get("user:123").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_delete_pattern_no_matches() {
        let cache = MemoryCache::new(TEST_MAX_ENTRIES);

        cache.set("user:123", b"value", None).await.unwrap();
        cache.set("user:456", b"value", None).await.unwrap();

        // Pattern with non-existent calendar ID
        let fake_id = Uuid::new_v4();
        let pattern = format!("calendar:{}:entries:*", fake_id);
        cache.delete_pattern(&pattern).await.unwrap();

        // All entries should still exist
        assert!(cache.get("user:123").await.unwrap().is_some());
        assert!(cache.get("user:456").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_delete_calendar_cleans_up_all() {
        let cache = MemoryCache::new(TEST_MAX_ENTRIES);

        // Create calendar with entries
        let calendar_id = Uuid::new_v4();
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        let metadata_key = calendar_key(calendar_id);
        let entries_key = calendar_entries_key(calendar_id, start, end);

        // Set values
        cache
            .set(&metadata_key, b"calendar data", None)
            .await
            .unwrap();
        cache
            .set(&entries_key, b"entries data", None)
            .await
            .unwrap();

        // Verify entries key is tracked
        {
            let tracking = cache.tracking.read().await;
            assert!(tracking.get(&calendar_id).is_some());
            assert!(tracking.get(&calendar_id).unwrap().contains(&entries_key));
        }

        // Delete the calendar metadata key - should clean up everything
        cache.delete(&metadata_key).await.unwrap();

        // Verify everything is gone
        assert!(cache.get(&metadata_key).await.unwrap().is_none());
        assert!(cache.get(&entries_key).await.unwrap().is_none());

        // Verify tracking set is cleaned up
        {
            let tracking = cache.tracking.read().await;
            assert!(tracking.get(&calendar_id).is_none());
        }
    }

    #[tokio::test]
    async fn test_delete_entry_removes_from_tracking() {
        let cache = MemoryCache::new(TEST_MAX_ENTRIES);

        let calendar_id = Uuid::new_v4();
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        let entries_key = calendar_entries_key(calendar_id, start, end);

        // Set entry
        cache
            .set(&entries_key, b"entries data", None)
            .await
            .unwrap();

        // Verify it's tracked
        {
            let tracking = cache.tracking.read().await;
            assert!(tracking.get(&calendar_id).unwrap().contains(&entries_key));
        }

        // Delete the entry directly
        cache.delete(&entries_key).await.unwrap();

        // Verify it's removed from tracking (and tracking set cleaned up since empty)
        {
            let tracking = cache.tracking.read().await;
            assert!(tracking.get(&calendar_id).is_none());
        }
    }

    #[tokio::test]
    async fn test_delete_pattern_non_calendar_falls_back() {
        let cache = MemoryCache::new(TEST_MAX_ENTRIES);

        cache
            .set("user:123:profile", b"value1", None)
            .await
            .unwrap();
        cache
            .set("user:456:profile", b"value2", None)
            .await
            .unwrap();
        cache
            .set("user:123:settings", b"value3", None)
            .await
            .unwrap();

        // Delete with non-calendar pattern (falls back to full iteration)
        cache.delete_pattern("user:123:*").await.unwrap();

        // Matching keys should be gone
        assert!(cache.get("user:123:profile").await.unwrap().is_none());
        assert!(cache.get("user:123:settings").await.unwrap().is_none());

        // Non-matching key should remain
        assert!(cache.get("user:456:profile").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_overwrite_value() {
        let cache = MemoryCache::new(TEST_MAX_ENTRIES);
        let key = "test:overwrite";

        cache.set(key, b"first", None).await.unwrap();
        cache.set(key, b"second", None).await.unwrap();

        let result = cache.get(key).await.unwrap();
        assert_eq!(result, Some(b"second".to_vec()));
    }

    #[tokio::test]
    async fn test_no_ttl_never_expires() {
        let cache = MemoryCache::new(TEST_MAX_ENTRIES);
        let key = "test:no-ttl";
        let value = b"persistent";

        cache.set(key, value, None).await.unwrap();

        // Even after a small delay, should still exist
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(cache.get(key).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        // Create a cache with only 3 entries max
        let cache = MemoryCache::new(3);

        // Insert 3 entries
        cache.set("key1", b"value1", None).await.unwrap();
        cache.set("key2", b"value2", None).await.unwrap();
        cache.set("key3", b"value3", None).await.unwrap();

        // All 3 should exist
        assert!(cache.get("key1").await.unwrap().is_some());
        assert!(cache.get("key2").await.unwrap().is_some());
        assert!(cache.get("key3").await.unwrap().is_some());

        // Access key1 to make it recently used
        cache.get("key1").await.unwrap();

        // Insert a 4th entry - should evict key2 (least recently used)
        cache.set("key4", b"value4", None).await.unwrap();

        // key1 should still exist (was recently accessed)
        assert!(cache.get("key1").await.unwrap().is_some());
        // key2 should be evicted (least recently used)
        assert!(cache.get("key2").await.unwrap().is_none());
        // key3 and key4 should exist
        assert!(cache.get("key3").await.unwrap().is_some());
        assert!(cache.get("key4").await.unwrap().is_some());
    }

    #[tokio::test]
    #[should_panic(expected = "max_entries must be > 0")]
    async fn test_zero_max_entries_panics() {
        let _ = MemoryCache::new(0);
    }
}
