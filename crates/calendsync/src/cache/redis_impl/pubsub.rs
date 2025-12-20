//! Redis pub/sub implementation.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::StreamExt;
use redis::AsyncCommands;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use calendsync_core::cache::{calendar_channel, CacheError, CachePubSub, Result};
use calendsync_core::calendar::CalendarEvent;

use super::error::map_redis_error;

/// Redis pub/sub backend for real-time calendar event broadcasting.
pub struct RedisPubSub {
    client: redis::Client,
    subscriptions: Arc<RwLock<HashMap<Uuid, broadcast::Sender<CalendarEvent>>>>,
}

impl RedisPubSub {
    /// Creates a new Redis pub/sub connection.
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

        // Verify connection by getting a connection
        let _ = client
            .get_multiplexed_async_connection()
            .await
            .map_err(map_redis_error)?;

        Ok(Self {
            client,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

#[async_trait]
impl CachePubSub for RedisPubSub {
    async fn publish(&self, calendar_id: Uuid, event: &CalendarEvent) -> Result<()> {
        let channel = calendar_channel(calendar_id);

        // Serialize the event to JSON
        let payload =
            serde_json::to_string(event).map_err(|e| CacheError::Serialization(e.to_string()))?;

        // Get a connection and publish
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(map_redis_error)?;

        conn.publish::<_, _, ()>(&channel, &payload)
            .await
            .map_err(|e| CacheError::PublishFailed(e.to_string()))?;

        Ok(())
    }

    async fn subscribe(&self, calendar_id: Uuid) -> Result<broadcast::Receiver<CalendarEvent>> {
        // Check if we already have a subscription for this calendar
        {
            let subscriptions = self.subscriptions.read().await;
            if let Some(sender) = subscriptions.get(&calendar_id) {
                return Ok(sender.subscribe());
            }
        }

        // Create a new broadcast channel
        let (tx, rx) = broadcast::channel(100);

        // Store the sender
        {
            let mut subscriptions = self.subscriptions.write().await;
            // Double-check in case another task created it
            if let Some(sender) = subscriptions.get(&calendar_id) {
                return Ok(sender.subscribe());
            }
            subscriptions.insert(calendar_id, tx.clone());
        }

        // Spawn a background task to handle Redis subscription
        let channel = calendar_channel(calendar_id);
        let client = self.client.clone();
        let subscriptions = Arc::clone(&self.subscriptions);

        tokio::spawn(async move {
            if let Err(e) =
                run_subscription_loop(client, channel, calendar_id, tx, subscriptions).await
            {
                tracing::error!(
                    "Redis subscription error for calendar {}: {}",
                    calendar_id,
                    e
                );
            }
        });

        Ok(rx)
    }
}

/// Runs the Redis subscription loop, forwarding messages to the broadcast channel.
async fn run_subscription_loop(
    client: redis::Client,
    channel: String,
    calendar_id: Uuid,
    tx: broadcast::Sender<CalendarEvent>,
    subscriptions: Arc<RwLock<HashMap<Uuid, broadcast::Sender<CalendarEvent>>>>,
) -> Result<()> {
    let mut pubsub = client.get_async_pubsub().await.map_err(map_redis_error)?;

    pubsub.subscribe(&channel).await.map_err(map_redis_error)?;

    let mut stream = pubsub.on_message();

    loop {
        match stream.next().await {
            Some(msg) => {
                let payload: String = msg.get_payload().map_err(map_redis_error)?;

                match serde_json::from_str::<CalendarEvent>(&payload) {
                    Ok(event) => {
                        // Send to broadcast channel
                        // Ignore send errors (no receivers)
                        let _ = tx.send(event);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to deserialize calendar event: {} - payload: {}",
                            e,
                            payload
                        );
                    }
                }
            }
            None => {
                // Stream ended, clean up subscription
                tracing::info!(
                    "Redis subscription stream ended for calendar {}",
                    calendar_id
                );
                break;
            }
        }
    }

    // Clean up subscription on exit
    let mut subs = subscriptions.write().await;
    subs.remove(&calendar_id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::time::Duration;

    /// Helper to get Redis URL from environment.
    fn redis_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
    }

    /// Skip test if Redis not available.
    async fn get_test_pubsub() -> Option<RedisPubSub> {
        RedisPubSub::new(&redis_url()).await.ok()
    }

    fn create_test_event() -> CalendarEvent {
        use calendsync_core::calendar::CalendarEntry;

        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let entry = CalendarEntry::all_day(calendar_id, "Test Event", date);
        CalendarEvent::entry_added(entry)
    }

    #[tokio::test]
    async fn test_redis_pubsub_creation() {
        let result = RedisPubSub::new(&redis_url()).await;
        if result.is_err() {
            eprintln!("Skipping test: Redis not available");
            return;
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_redis_pubsub_publish_and_receive() {
        let Some(pubsub) = get_test_pubsub().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let calendar_id = Uuid::new_v4();
        let event = create_test_event();

        // Subscribe first
        let mut rx = pubsub.subscribe(calendar_id).await.unwrap();

        // Give the subscription time to establish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Publish event
        pubsub.publish(calendar_id, &event).await.unwrap();

        // Receive with timeout
        let received = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;

        match received {
            Ok(Ok(received_event)) => {
                assert_eq!(received_event, event);
            }
            Ok(Err(e)) => {
                panic!("Receive error: {:?}", e);
            }
            Err(_) => {
                panic!("Timeout waiting for event");
            }
        }
    }

    #[tokio::test]
    async fn test_redis_pubsub_multiple_subscribers() {
        let Some(pubsub) = get_test_pubsub().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let calendar_id = Uuid::new_v4();
        let event = create_test_event();

        // Create two subscribers
        let mut rx1 = pubsub.subscribe(calendar_id).await.unwrap();
        let mut rx2 = pubsub.subscribe(calendar_id).await.unwrap();

        // Give subscriptions time to establish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Publish event
        pubsub.publish(calendar_id, &event).await.unwrap();

        // Both should receive
        let timeout = Duration::from_secs(2);
        let received1 = tokio::time::timeout(timeout, rx1.recv()).await;
        let received2 = tokio::time::timeout(timeout, rx2.recv()).await;

        assert!(received1.is_ok());
        assert!(received2.is_ok());
    }

    #[tokio::test]
    async fn test_redis_pubsub_different_calendars() {
        let Some(pubsub) = get_test_pubsub().await else {
            eprintln!("Skipping test: Redis not available");
            return;
        };

        let calendar_id_1 = Uuid::new_v4();
        let calendar_id_2 = Uuid::new_v4();
        let event = create_test_event();

        // Subscribe to calendar 1 only
        let mut rx1 = pubsub.subscribe(calendar_id_1).await.unwrap();

        // Give subscription time to establish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Publish to calendar 2 (should not be received by rx1)
        pubsub.publish(calendar_id_2, &event).await.unwrap();

        // Should timeout since event was for different calendar
        let received = tokio::time::timeout(Duration::from_millis(200), rx1.recv()).await;

        assert!(
            received.is_err(),
            "Should not receive event for different calendar"
        );
    }
}
