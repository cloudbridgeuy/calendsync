//! Pure functions for serializing/deserializing domain types to/from cache bytes.
//!
//! These functions use JSON serialization for cache storage, providing human-readable
//! cache values that are easy to debug and inspect.

use crate::calendar::{Calendar, CalendarEntry};
use thiserror::Error;

/// Errors that can occur during cache serialization/deserialization.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SerializationError {
    /// Failed to serialize a value to bytes.
    #[error("Failed to serialize: {0}")]
    SerializeFailed(String),
    /// Failed to deserialize bytes to a value.
    #[error("Failed to deserialize: {0}")]
    DeserializeFailed(String),
}

/// Result type for serialization operations.
pub type Result<T> = std::result::Result<T, SerializationError>;

/// Serializes a calendar entry to JSON bytes.
///
/// # Arguments
/// * `entry` - The calendar entry to serialize
///
/// # Returns
/// JSON-encoded bytes representing the entry
pub fn serialize_entry(entry: &CalendarEntry) -> Result<Vec<u8>> {
    serde_json::to_vec(entry).map_err(|e| SerializationError::SerializeFailed(e.to_string()))
}

/// Deserializes JSON bytes to a calendar entry.
///
/// # Arguments
/// * `bytes` - JSON-encoded bytes
///
/// # Returns
/// The deserialized calendar entry
pub fn deserialize_entry(bytes: &[u8]) -> Result<CalendarEntry> {
    serde_json::from_slice(bytes).map_err(|e| SerializationError::DeserializeFailed(e.to_string()))
}

/// Serializes a slice of calendar entries to JSON bytes.
///
/// # Arguments
/// * `entries` - The calendar entries to serialize
///
/// # Returns
/// JSON-encoded bytes representing the entries array
pub fn serialize_entries(entries: &[CalendarEntry]) -> Result<Vec<u8>> {
    serde_json::to_vec(entries).map_err(|e| SerializationError::SerializeFailed(e.to_string()))
}

/// Deserializes JSON bytes to a vector of calendar entries.
///
/// # Arguments
/// * `bytes` - JSON-encoded bytes
///
/// # Returns
/// The deserialized vector of calendar entries
pub fn deserialize_entries(bytes: &[u8]) -> Result<Vec<CalendarEntry>> {
    serde_json::from_slice(bytes).map_err(|e| SerializationError::DeserializeFailed(e.to_string()))
}

/// Serializes a calendar to JSON bytes.
///
/// # Arguments
/// * `calendar` - The calendar to serialize
///
/// # Returns
/// JSON-encoded bytes representing the calendar
pub fn serialize_calendar(calendar: &Calendar) -> Result<Vec<u8>> {
    serde_json::to_vec(calendar).map_err(|e| SerializationError::SerializeFailed(e.to_string()))
}

/// Deserializes JSON bytes to a calendar.
///
/// # Arguments
/// * `bytes` - JSON-encoded bytes
///
/// # Returns
/// The deserialized calendar
pub fn deserialize_calendar(bytes: &[u8]) -> Result<Calendar> {
    serde_json::from_slice(bytes).map_err(|e| SerializationError::DeserializeFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
    use uuid::Uuid;

    fn fixed_timestamp() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 6, 15, 10, 30, 0).unwrap()
    }

    fn test_calendar_id() -> Uuid {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    }

    fn test_entry_id() -> Uuid {
        Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap()
    }

    #[test]
    fn test_roundtrip_entry() {
        let calendar_id = test_calendar_id();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = CalendarEntry::all_day(calendar_id, "Test Event", date)
            .with_id(test_entry_id())
            .with_created_at(fixed_timestamp())
            .with_updated_at(fixed_timestamp());

        let bytes = serialize_entry(&entry).expect("serialize should succeed");
        let deserialized = deserialize_entry(&bytes).expect("deserialize should succeed");

        assert_eq!(entry, deserialized);
    }

    #[test]
    fn test_roundtrip_entries_vec() {
        let calendar_id = test_calendar_id();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entries = vec![
            CalendarEntry::all_day(calendar_id, "Event 1", date)
                .with_id(test_entry_id())
                .with_created_at(fixed_timestamp())
                .with_updated_at(fixed_timestamp()),
            CalendarEntry::task(calendar_id, "Task 1", date, false)
                .with_id(Uuid::parse_str("7ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap())
                .with_created_at(fixed_timestamp())
                .with_updated_at(fixed_timestamp()),
        ];

        let bytes = serialize_entries(&entries).expect("serialize should succeed");
        let deserialized = deserialize_entries(&bytes).expect("deserialize should succeed");

        assert_eq!(entries, deserialized);
    }

    #[test]
    fn test_roundtrip_calendar() {
        let calendar = Calendar::new("Work Calendar", "#3B82F6")
            .with_id(test_calendar_id())
            .with_created_at(fixed_timestamp())
            .with_updated_at(fixed_timestamp());

        let bytes = serialize_calendar(&calendar).expect("serialize should succeed");
        let deserialized = deserialize_calendar(&bytes).expect("deserialize should succeed");

        assert_eq!(calendar, deserialized);
    }

    #[test]
    fn test_deserialize_entry_malformed_bytes() {
        let malformed = b"not valid json";
        let result = deserialize_entry(malformed);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SerializationError::DeserializeFailed(_)));
    }

    #[test]
    fn test_deserialize_entries_malformed_bytes() {
        let malformed = b"{\"invalid\": true}";
        let result = deserialize_entries(malformed);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SerializationError::DeserializeFailed(_)));
    }

    #[test]
    fn test_deserialize_calendar_malformed_bytes() {
        let malformed = b"[1, 2, 3]";
        let result = deserialize_calendar(malformed);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SerializationError::DeserializeFailed(_)));
    }

    #[test]
    fn test_serialize_empty_entries_vec() {
        let entries: Vec<CalendarEntry> = vec![];

        let bytes = serialize_entries(&entries).expect("serialize should succeed");
        let deserialized = deserialize_entries(&bytes).expect("deserialize should succeed");

        assert!(deserialized.is_empty());
        assert_eq!(bytes, b"[]");
    }

    #[test]
    fn test_entry_with_all_optional_fields() {
        let calendar_id = test_calendar_id();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let start_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let end_time = NaiveTime::from_hms_opt(10, 30, 0).unwrap();

        let entry = CalendarEntry::timed(calendar_id, "Meeting", date, start_time, end_time)
            .with_id(test_entry_id())
            .with_description("Team sync meeting")
            .with_location("Conference Room A")
            .with_color("#EF4444")
            .with_created_at(fixed_timestamp())
            .with_updated_at(fixed_timestamp());

        let bytes = serialize_entry(&entry).expect("serialize should succeed");
        let deserialized = deserialize_entry(&bytes).expect("deserialize should succeed");

        assert_eq!(entry, deserialized);
        assert_eq!(
            deserialized.description,
            Some("Team sync meeting".to_string())
        );
        assert_eq!(deserialized.location, Some("Conference Room A".to_string()));
        assert_eq!(deserialized.color, Some("#EF4444".to_string()));
    }

    #[test]
    fn test_entry_with_minimal_fields() {
        let calendar_id = test_calendar_id();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        let entry = CalendarEntry::all_day(calendar_id, "Simple Event", date)
            .with_id(test_entry_id())
            .with_created_at(fixed_timestamp())
            .with_updated_at(fixed_timestamp());

        let bytes = serialize_entry(&entry).expect("serialize should succeed");
        let deserialized = deserialize_entry(&bytes).expect("deserialize should succeed");

        assert_eq!(entry, deserialized);
        assert!(deserialized.description.is_none());
        assert!(deserialized.location.is_none());
        assert!(deserialized.color.is_none());
    }

    #[test]
    fn test_calendar_with_description() {
        let calendar = Calendar::new("Personal", "#10B981")
            .with_id(test_calendar_id())
            .with_description("My personal calendar for appointments")
            .with_created_at(fixed_timestamp())
            .with_updated_at(fixed_timestamp());

        let bytes = serialize_calendar(&calendar).expect("serialize should succeed");
        let deserialized = deserialize_calendar(&bytes).expect("deserialize should succeed");

        assert_eq!(calendar, deserialized);
        assert_eq!(
            deserialized.description,
            Some("My personal calendar for appointments".to_string())
        );
    }

    #[test]
    fn test_calendar_without_description() {
        let calendar = Calendar::new("Work", "#3B82F6")
            .with_id(test_calendar_id())
            .with_created_at(fixed_timestamp())
            .with_updated_at(fixed_timestamp());

        let bytes = serialize_calendar(&calendar).expect("serialize should succeed");
        let deserialized = deserialize_calendar(&bytes).expect("deserialize should succeed");

        assert_eq!(calendar, deserialized);
        assert!(deserialized.description.is_none());
    }
}
