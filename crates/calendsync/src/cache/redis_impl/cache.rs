//! Redis cache implementation.
//!
//! Uses set-based key tracking for efficient pattern deletion without SCAN.
//! Calendar entry keys are tracked in Redis Sets keyed by calendar ID.
//!
//! # Non-Atomicity Safety
//!
//! The operations in this module (especially `delete` and `delete_pattern`) are
//! not atomic - they involve multiple Redis commands. However, this is safe because:
//!
//! - **SREM on non-existent key**: If a key is deleted but the process crashes before
//!   SREM, the tracking set will contain a stale reference. This is harmless because
//!   SREM on a non-existent member is a no-op, and DEL on a non-existent key is also safe.
//!
//! - **Orphaned entries in tracking set**: If keys are added to tracking but the actual
//!   SET fails, the tracking set may reference non-existent keys. This is harmless because
//!   delete_pattern will simply try to delete keys that don't exist.
//!
//! - **Partial deletion**: If delete_pattern deletes some keys but crashes before
//!   completing, subsequent calls will finish the cleanup safely.
//!
//! The worst case is temporary inconsistency, not data corruption or lost writes.

use std::time::Duration;

use async_trait::async_trait;
use redis::AsyncCommands;

use calendsync_core::cache::{
    calendar_tracking_key, extract_calendar_id_from_key, extract_calendar_id_from_pattern,
    is_calendar_entry_key, is_calendar_metadata_key, pattern_matches, Cache, Result,
};

use super::error::map_redis_error;

/// Redis cache backend using connection manager for pooling.
///
/// Calendar entry keys are automatically tracked in Redis Sets to enable
/// efficient pattern-based deletion without using SCAN operations.
pub struct RedisCache {
    conn: redis::aio::ConnectionManager,
}

impl RedisCache {
    /// Creates a new Redis cache connection.
    ///
    /// # Arguments
    ///
    /// * `url` - Redis connection URL (e.g., "redis://localhost:6379")
    ///
    /// # Errors
    ///
    /// Returns `CacheError::ConnectionFailed` if the connection cannot be established.
    pub async fn new(url: &str) -> Result<Self> {
        let client = redis::Client::open(url).map_err(map_redis_error)?;
        let conn = redis::aio::ConnectionManager::new(client)
            .await
            .map_err(map_redis_error)?;
        Ok(Self { conn })
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut conn = self.conn.clone();
        let result: Option<Vec<u8>> = conn.get(key).await.map_err(map_redis_error)?;
        Ok(result)
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<()> {
        let mut conn = self.conn.clone();

        // Set the value
        match ttl {
            Some(duration) => {
                let seconds = duration.as_secs().max(1);
                conn.set_ex::<_, _, ()>(key, value, seconds)
                    .await
                    .map_err(map_redis_error)?;
            }
            None => {
                conn.set::<_, _, ()>(key, value)
                    .await
                    .map_err(map_redis_error)?;
            }
        }

        // Track calendar entry keys in the calendar's tracking set
        if is_calendar_entry_key(key) {
            if let Some(calendar_id) = extract_calendar_id_from_key(key) {
                let tracking_key = calendar_tracking_key(calendar_id);
                conn.sadd::<_, _, ()>(&tracking_key, key)
                    .await
                    .map_err(map_redis_error)?;
            }
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.conn.clone();

        // Note: The following operations are not atomic, but this is safe.
        // See module-level documentation for details on non-atomicity safety.

        // Case 1: Deleting a calendar metadata key - full cleanup
        if is_calendar_metadata_key(key) {
            if let Some(calendar_id) = extract_calendar_id_from_key(key) {
                let tracking_key = calendar_tracking_key(calendar_id);

                // Get all tracked keys for this calendar
                let tracked_keys: Vec<String> = conn
                    .smembers(&tracking_key)
                    .await
                    .map_err(map_redis_error)?;

                // Delete all tracked keys
                if !tracked_keys.is_empty() {
                    conn.del::<_, ()>(&tracked_keys)
                        .await
                        .map_err(map_redis_error)?;
                }

                // Delete the tracking set
                conn.del::<_, ()>(&tracking_key)
                    .await
                    .map_err(map_redis_error)?;
            }
        }
        // Case 2: Deleting a calendar entry key - remove from tracking
        else if is_calendar_entry_key(key) {
            if let Some(calendar_id) = extract_calendar_id_from_key(key) {
                let tracking_key = calendar_tracking_key(calendar_id);
                conn.srem::<_, _, ()>(&tracking_key, key)
                    .await
                    .map_err(map_redis_error)?;
            }
        }
        // Case 3: Non-calendar key - just delete

        // Delete the key itself
        conn.del::<_, ()>(key).await.map_err(map_redis_error)?;

        Ok(())
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        // Extract calendar ID from pattern
        let Some(calendar_id) = extract_calendar_id_from_pattern(pattern) else {
            // Non-calendar pattern - no-op (we only track calendar keys)
            return Ok(());
        };

        let mut conn = self.conn.clone();
        let tracking_key = calendar_tracking_key(calendar_id);

        // Get all tracked keys for this calendar
        let tracked_keys: Vec<String> = conn
            .smembers(&tracking_key)
            .await
            .map_err(map_redis_error)?;

        // Filter keys that match the pattern
        let keys_to_delete: Vec<&String> = tracked_keys
            .iter()
            .filter(|k| pattern_matches(pattern, k))
            .collect();

        if !keys_to_delete.is_empty() {
            // Delete matching keys
            conn.del::<_, ()>(&keys_to_delete)
                .await
                .map_err(map_redis_error)?;

            // Remove from tracking set
            conn.srem::<_, _, ()>(&tracking_key, &keys_to_delete)
                .await
                .map_err(map_redis_error)?;
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
    use uuid::Uuid;

    /// Helper to get Redis URL from environment.
    fn redis_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
    }

    /// Skip test if Redis not available.
    async fn get_test_cache() -> Option<RedisCache> {
        RedisCache::new(&redis_url()).await.ok()
    }

    /// Generate a unique test key to avoid conflicts.
    fn test_key(suffix: &str) -> String {
        format!("test:redis_cache:{}:{}", Uuid::new_v4(), suffix)
    }

    #[tokio::test]
    async fn test_redis_set_and_get() {
        let Some(cache) = get_test_cache().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let key = test_key("set_get");
        let value = b"hello world";

        // Set the value
        cache.set(&key, value, None).await.unwrap();

        // Get the value
        let result = cache.get(&key).await.unwrap();
        assert_eq!(result, Some(value.to_vec()));

        // Clean up
        cache.delete(&key).await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_get_nonexistent() {
        let Some(cache) = get_test_cache().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let key = test_key("nonexistent");
        let result = cache.get(&key).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_redis_delete() {
        let Some(cache) = get_test_cache().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let key = test_key("delete");
        let value = b"to be deleted";

        // Set the value
        cache.set(&key, value, None).await.unwrap();

        // Verify it exists
        assert!(cache.get(&key).await.unwrap().is_some());

        // Delete it
        cache.delete(&key).await.unwrap();

        // Verify it's gone
        assert!(cache.get(&key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_redis_ttl() {
        let Some(cache) = get_test_cache().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let key = test_key("ttl");
        let value = b"expiring value";

        // Set with 1 second TTL
        cache
            .set(&key, value, Some(Duration::from_secs(1)))
            .await
            .unwrap();

        // Verify it exists immediately
        assert!(cache.get(&key).await.unwrap().is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(1500)).await;

        // Verify it's expired
        assert!(cache.get(&key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_redis_delete_pattern() {
        let Some(cache) = get_test_cache().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        // Use proper calendar-formatted keys so they get tracked
        let calendar_id = Uuid::new_v4();
        let start1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end1 = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let start2 = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
        let end2 = NaiveDate::from_ymd_opt(2024, 2, 28).unwrap();

        let key1 = calendar_entries_key(calendar_id, start1, end1);
        let key2 = calendar_entries_key(calendar_id, start2, end2);
        let key3 = calendar_key(calendar_id); // Metadata key, not tracked

        // Set multiple values
        cache.set(&key1, b"value1", None).await.unwrap();
        cache.set(&key2, b"value2", None).await.unwrap();
        cache.set(&key3, b"value3", None).await.unwrap();

        // Delete pattern matching entries (should delete key1 and key2)
        let pattern = format!("calendar:{}:entries:*", calendar_id);
        cache.delete_pattern(&pattern).await.unwrap();

        // Verify entries keys are deleted
        assert!(cache.get(&key1).await.unwrap().is_none());
        assert!(cache.get(&key2).await.unwrap().is_none());

        // Verify metadata key is still there (not matched by pattern)
        assert!(cache.get(&key3).await.unwrap().is_some());

        // Clean up
        cache.delete(&key3).await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_delete_calendar_cleans_up_all() {
        let Some(cache) = get_test_cache().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        // Create calendar with entries
        let calendar_id = Uuid::new_v4();
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        let metadata_key = calendar_key(calendar_id);
        let entries_key = calendar_entries_key(calendar_id, start, end);
        let tracking_key = calendar_tracking_key(calendar_id);

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
        let mut conn = cache.conn.clone();
        let tracked: Vec<String> = conn.smembers(&tracking_key).await.unwrap();
        assert!(tracked.contains(&entries_key));

        // Delete the calendar metadata key - should clean up everything
        cache.delete(&metadata_key).await.unwrap();

        // Verify everything is gone
        assert!(cache.get(&metadata_key).await.unwrap().is_none());
        assert!(cache.get(&entries_key).await.unwrap().is_none());

        // Verify tracking set is gone
        let tracked_after: Vec<String> = conn.smembers(&tracking_key).await.unwrap();
        assert!(tracked_after.is_empty());
    }

    #[tokio::test]
    async fn test_redis_delete_entry_removes_from_tracking() {
        let Some(cache) = get_test_cache().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let calendar_id = Uuid::new_v4();
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        let entries_key = calendar_entries_key(calendar_id, start, end);
        let tracking_key = calendar_tracking_key(calendar_id);

        // Set entry
        cache
            .set(&entries_key, b"entries data", None)
            .await
            .unwrap();

        // Verify it's tracked
        let mut conn = cache.conn.clone();
        let tracked: Vec<String> = conn.smembers(&tracking_key).await.unwrap();
        assert!(tracked.contains(&entries_key));

        // Delete the entry directly
        cache.delete(&entries_key).await.unwrap();

        // Verify it's removed from tracking
        let tracked_after: Vec<String> = conn.smembers(&tracking_key).await.unwrap();
        assert!(!tracked_after.contains(&entries_key));

        // Clean up tracking set
        conn.del::<_, ()>(&tracking_key).await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_delete_pattern_non_calendar_is_noop() {
        let Some(cache) = get_test_cache().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let key = test_key("noop");
        cache.set(&key, b"value", None).await.unwrap();

        // Delete with non-calendar pattern should be a no-op
        cache.delete_pattern("user:*").await.unwrap();

        // Key should still exist
        assert!(cache.get(&key).await.unwrap().is_some());

        // Clean up
        cache.delete(&key).await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_overwrite() {
        let Some(cache) = get_test_cache().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let key = test_key("overwrite");

        // Set initial value
        cache.set(&key, b"initial", None).await.unwrap();
        assert_eq!(cache.get(&key).await.unwrap(), Some(b"initial".to_vec()));

        // Overwrite with new value
        cache.set(&key, b"updated", None).await.unwrap();
        assert_eq!(cache.get(&key).await.unwrap(), Some(b"updated".to_vec()));

        // Clean up
        cache.delete(&key).await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_binary_data() {
        let Some(cache) = get_test_cache().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let key = test_key("binary");
        let value: Vec<u8> = (0..=255).collect();

        // Set binary data
        cache.set(&key, &value, None).await.unwrap();

        // Get and verify
        let result = cache.get(&key).await.unwrap();
        assert_eq!(result, Some(value));

        // Clean up
        cache.delete(&key).await.unwrap();
    }
}
