//! In-memory pub/sub implementation.
//!
//! Provides a thread-safe pub/sub mechanism for calendar events using
//! tokio broadcast channels.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use calendsync_core::cache::{CachePubSub, Result};
use calendsync_core::calendar::CalendarEvent;

/// Channel capacity for pub/sub messages.
const CHANNEL_CAPACITY: usize = 100;

/// In-memory pub/sub implementation.
///
/// Thread-safe pub/sub using tokio broadcast channels.
/// Each calendar has its own channel for targeted event delivery.
#[derive(Debug, Clone)]
pub struct MemoryPubSub {
    channels: Arc<RwLock<HashMap<Uuid, broadcast::Sender<CalendarEvent>>>>,
}

impl MemoryPubSub {
    /// Creates a new empty pub/sub instance.
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Gets or creates a channel for the given calendar ID.
    async fn get_or_create_channel(&self, calendar_id: Uuid) -> broadcast::Sender<CalendarEvent> {
        // Try read lock first to avoid write contention
        {
            let channels = self.channels.read().await;
            if let Some(sender) = channels.get(&calendar_id) {
                return sender.clone();
            }
        }

        // Need to create a new channel
        let mut channels = self.channels.write().await;

        // Double-check after acquiring write lock
        if let Some(sender) = channels.get(&calendar_id) {
            return sender.clone();
        }

        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        channels.insert(calendar_id, sender.clone());
        sender
    }
}

impl Default for MemoryPubSub {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CachePubSub for MemoryPubSub {
    async fn publish(&self, calendar_id: Uuid, event: &CalendarEvent) -> Result<()> {
        let sender = self.get_or_create_channel(calendar_id).await;

        // Send the event. If there are no receivers, that's fine -
        // it just means no one is subscribed to this calendar.
        // We clone the event since broadcast::send takes ownership.
        let _ = sender.send(event.clone());

        Ok(())
    }

    async fn subscribe(&self, calendar_id: Uuid) -> Result<broadcast::Receiver<CalendarEvent>> {
        let sender = self.get_or_create_channel(calendar_id).await;
        Ok(sender.subscribe())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use calendsync_core::calendar::CalendarEntry;
    use chrono::NaiveDate;

    fn create_test_entry(calendar_id: Uuid) -> CalendarEntry {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        CalendarEntry::all_day(calendar_id, "Test Event", date)
    }

    #[tokio::test]
    async fn test_publish_and_subscribe() {
        let pubsub = MemoryPubSub::new();
        let calendar_id = Uuid::new_v4();
        let entry = create_test_entry(calendar_id);
        let event = CalendarEvent::entry_added(entry.clone());

        // Subscribe first
        let mut receiver = pubsub.subscribe(calendar_id).await.unwrap();

        // Publish event
        pubsub.publish(calendar_id, &event).await.unwrap();

        // Receive event
        let received = receiver.recv().await.unwrap();

        match (&event, &received) {
            (
                CalendarEvent::EntryAdded {
                    entry: e1,
                    date: d1,
                },
                CalendarEvent::EntryAdded {
                    entry: e2,
                    date: d2,
                },
            ) => {
                assert_eq!(e1.id, e2.id);
                assert_eq!(e1.title, e2.title);
                assert_eq!(d1, d2);
            }
            _ => panic!("Event types don't match"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let pubsub = MemoryPubSub::new();
        let calendar_id = Uuid::new_v4();
        let entry = create_test_entry(calendar_id);
        let event = CalendarEvent::entry_added(entry);

        // Create two subscribers
        let mut receiver1 = pubsub.subscribe(calendar_id).await.unwrap();
        let mut receiver2 = pubsub.subscribe(calendar_id).await.unwrap();

        // Publish event
        pubsub.publish(calendar_id, &event).await.unwrap();

        // Both should receive the event
        let received1 = receiver1.recv().await.unwrap();
        let received2 = receiver2.recv().await.unwrap();

        // Verify both received the same event type
        assert!(matches!(received1, CalendarEvent::EntryAdded { .. }));
        assert!(matches!(received2, CalendarEvent::EntryAdded { .. }));
    }

    #[tokio::test]
    async fn test_subscribe_different_calendars() {
        let pubsub = MemoryPubSub::new();
        let calendar_id_1 = Uuid::new_v4();
        let calendar_id_2 = Uuid::new_v4();

        let entry1 = create_test_entry(calendar_id_1);
        let entry2 = create_test_entry(calendar_id_2);
        let event1 = CalendarEvent::entry_added(entry1.clone());
        let event2 = CalendarEvent::entry_added(entry2.clone());

        // Subscribe to different calendars
        let mut receiver1 = pubsub.subscribe(calendar_id_1).await.unwrap();
        let mut receiver2 = pubsub.subscribe(calendar_id_2).await.unwrap();

        // Publish to calendar 1
        pubsub.publish(calendar_id_1, &event1).await.unwrap();

        // Only receiver1 should get the event
        let received1 = receiver1.recv().await.unwrap();
        match received1 {
            CalendarEvent::EntryAdded { entry, .. } => {
                assert_eq!(entry.id, entry1.id);
            }
            _ => panic!("Expected EntryAdded event"),
        }

        // Publish to calendar 2
        pubsub.publish(calendar_id_2, &event2).await.unwrap();

        // Only receiver2 should get this event
        let received2 = receiver2.recv().await.unwrap();
        match received2 {
            CalendarEvent::EntryAdded { entry, .. } => {
                assert_eq!(entry.id, entry2.id);
            }
            _ => panic!("Expected EntryAdded event"),
        }
    }

    #[tokio::test]
    async fn test_publish_no_subscribers() {
        let pubsub = MemoryPubSub::new();
        let calendar_id = Uuid::new_v4();
        let entry = create_test_entry(calendar_id);
        let event = CalendarEvent::entry_added(entry);

        // Publish without any subscribers - should not error
        let result = pubsub.publish(calendar_id, &event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_channel_reuse() {
        let pubsub = MemoryPubSub::new();
        let calendar_id = Uuid::new_v4();

        // Create first subscriber
        let _receiver1 = pubsub.subscribe(calendar_id).await.unwrap();

        // Verify only one channel exists
        {
            let channels = pubsub.channels.read().await;
            assert_eq!(channels.len(), 1);
        }

        // Create second subscriber for same calendar
        let _receiver2 = pubsub.subscribe(calendar_id).await.unwrap();

        // Should still only have one channel
        {
            let channels = pubsub.channels.read().await;
            assert_eq!(channels.len(), 1);
        }
    }
}
