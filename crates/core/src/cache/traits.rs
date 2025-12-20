use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::calendar::CalendarEvent;

use super::Result;

/// Trait for basic cache operations.
#[async_trait]
pub trait Cache: Send + Sync {
    /// Gets a value from the cache by key.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Sets a value in the cache with an optional TTL.
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<()>;

    /// Deletes a value from the cache by key.
    async fn delete(&self, key: &str) -> Result<()>;

    /// Deletes all values matching a pattern (e.g., "calendar:*:entries:*").
    async fn delete_pattern(&self, pattern: &str) -> Result<()>;
}

/// Trait for cache pub/sub operations.
#[async_trait]
pub trait CachePubSub: Send + Sync {
    /// Publishes a calendar event to subscribers.
    async fn publish(&self, calendar_id: Uuid, event: &CalendarEvent) -> Result<()>;

    /// Subscribes to calendar events for a specific calendar.
    async fn subscribe(&self, calendar_id: Uuid) -> Result<broadcast::Receiver<CalendarEvent>>;
}

/// Combined trait for caches that support both basic operations and pub/sub.
pub trait FullCache: Cache + CachePubSub {}

impl<T: Cache + CachePubSub> FullCache for T {}
