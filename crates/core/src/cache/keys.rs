use chrono::NaiveDate;
use uuid::Uuid;

/// Returns the cache key for a single entry.
pub fn entry_key(entry_id: Uuid) -> String {
    format!("entry:{}", entry_id)
}

/// Returns the cache key for calendar entries within a date range.
pub fn calendar_entries_key(calendar_id: Uuid, start: NaiveDate, end: NaiveDate) -> String {
    format!("calendar:{}:entries:{}:{}", calendar_id, start, end)
}

/// Returns the pattern for matching all calendar entries keys.
pub fn calendar_entries_pattern(calendar_id: Uuid) -> String {
    format!("calendar:{}:entries:*", calendar_id)
}

/// Returns the cache key for a calendar.
pub fn calendar_key(calendar_id: Uuid) -> String {
    format!("calendar:{}", calendar_id)
}

/// Returns the cache key for a user.
pub fn user_key(user_id: Uuid) -> String {
    format!("user:{}", user_id)
}

/// Returns the pub/sub channel name for calendar events.
pub fn calendar_channel(calendar_id: Uuid) -> String {
    format!("channel:calendar:{}", calendar_id)
}

/// Returns the Redis Set key for tracking cache keys of a calendar.
///
/// This set contains all cache keys associated with a calendar (e.g., entry
/// date range queries) to enable efficient pattern-based deletion without
/// using Redis SCAN.
pub fn calendar_tracking_key(calendar_id: Uuid) -> String {
    format!("calendar:{}:_keys", calendar_id)
}

/// Extracts the calendar ID from a cache key, if present.
///
/// Returns `None` for non-calendar keys (e.g., `"user:123"`, `"entry:456"`).
///
/// # Examples
///
/// ```
/// use calendsync_core::cache::extract_calendar_id_from_key;
/// use uuid::Uuid;
///
/// let id = Uuid::nil();
/// let key = format!("calendar:{}:entries:2024-01-01:2024-01-31", id);
/// assert_eq!(extract_calendar_id_from_key(&key), Some(id));
///
/// assert_eq!(extract_calendar_id_from_key("user:123"), None);
/// ```
pub fn extract_calendar_id_from_key(key: &str) -> Option<Uuid> {
    let rest = key.strip_prefix("calendar:")?;
    let uuid_part = rest.split(':').next()?;
    Uuid::parse_str(uuid_part).ok()
}

/// Extracts the calendar ID from a cache pattern, if present.
///
/// Returns `None` for non-calendar patterns or patterns with wildcards
/// in the UUID position.
///
/// # Examples
///
/// ```
/// use calendsync_core::cache::extract_calendar_id_from_pattern;
/// use uuid::Uuid;
///
/// let id = Uuid::nil();
/// let pattern = format!("calendar:{}:entries:*", id);
/// assert_eq!(extract_calendar_id_from_pattern(&pattern), Some(id));
///
/// // Wildcard in UUID position - cannot extract
/// assert_eq!(extract_calendar_id_from_pattern("calendar:*:entries:*"), None);
/// ```
pub fn extract_calendar_id_from_pattern(pattern: &str) -> Option<Uuid> {
    let rest = pattern.strip_prefix("calendar:")?;
    let uuid_part = rest.split(':').next()?;
    // If UUID part contains wildcard, we can't extract a specific ID
    if uuid_part.contains('*') {
        return None;
    }
    Uuid::parse_str(uuid_part).ok()
}

/// Checks if a cache key is a calendar metadata key (e.g., `"calendar:{id}"`).
///
/// This is used to detect when a calendar is being deleted so we can
/// clean up its tracking set and cached entries.
pub fn is_calendar_metadata_key(key: &str) -> bool {
    if !key.starts_with("calendar:") {
        return false;
    }
    let rest = key.strip_prefix("calendar:").unwrap();
    // Calendar metadata key has format "calendar:{uuid}" with no additional segments
    // So after the UUID, there should be nothing left
    let parts: Vec<&str> = rest.split(':').collect();
    if parts.len() != 1 {
        return false;
    }
    // Verify it's a valid UUID
    Uuid::parse_str(parts[0]).is_ok()
}

/// Checks if a cache key is a calendar entry key (e.g., `"calendar:{id}:entries:..."`).
///
/// These keys should be tracked in the calendar's tracking set.
pub fn is_calendar_entry_key(key: &str) -> bool {
    if !key.starts_with("calendar:") {
        return false;
    }
    let rest = key.strip_prefix("calendar:").unwrap();
    let parts: Vec<&str> = rest.split(':').collect();
    // Must have at least UUID + "entries" + more
    if parts.len() < 3 {
        return false;
    }
    // Second segment should be "entries"
    parts[1] == "entries"
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use uuid::Uuid;

    fn test_uuid() -> Uuid {
        Uuid::nil()
    }

    #[test]
    fn test_entry_key() {
        let key = entry_key(test_uuid());
        assert_eq!(key, "entry:00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn test_calendar_entries_key() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let key = calendar_entries_key(test_uuid(), start, end);
        assert_eq!(
            key,
            "calendar:00000000-0000-0000-0000-000000000000:entries:2024-01-01:2024-01-31"
        );
    }

    #[test]
    fn test_calendar_entries_pattern() {
        let pattern = calendar_entries_pattern(test_uuid());
        assert_eq!(
            pattern,
            "calendar:00000000-0000-0000-0000-000000000000:entries:*"
        );
    }

    #[test]
    fn test_calendar_key() {
        let key = calendar_key(test_uuid());
        assert_eq!(key, "calendar:00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn test_user_key() {
        let key = user_key(test_uuid());
        assert_eq!(key, "user:00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn test_calendar_channel() {
        let channel = calendar_channel(test_uuid());
        assert_eq!(
            channel,
            "channel:calendar:00000000-0000-0000-0000-000000000000"
        );
    }

    #[test]
    fn test_calendar_tracking_key() {
        let key = calendar_tracking_key(test_uuid());
        assert_eq!(key, "calendar:00000000-0000-0000-0000-000000000000:_keys");
    }

    #[test]
    fn test_extract_calendar_id_from_key_entries() {
        let id = test_uuid();
        let key = format!("calendar:{}:entries:2024-01-01:2024-01-31", id);
        assert_eq!(extract_calendar_id_from_key(&key), Some(id));
    }

    #[test]
    fn test_extract_calendar_id_from_key_metadata() {
        let id = test_uuid();
        let key = format!("calendar:{}", id);
        assert_eq!(extract_calendar_id_from_key(&key), Some(id));
    }

    #[test]
    fn test_extract_calendar_id_from_key_non_calendar() {
        assert_eq!(extract_calendar_id_from_key("user:123"), None);
        assert_eq!(extract_calendar_id_from_key("entry:456"), None);
        assert_eq!(extract_calendar_id_from_key("random:key"), None);
    }

    #[test]
    fn test_extract_calendar_id_from_key_invalid_uuid() {
        assert_eq!(
            extract_calendar_id_from_key("calendar:not-a-uuid:entries"),
            None
        );
    }

    #[test]
    fn test_extract_calendar_id_from_pattern_valid() {
        let id = test_uuid();
        let pattern = format!("calendar:{}:entries:*", id);
        assert_eq!(extract_calendar_id_from_pattern(&pattern), Some(id));
    }

    #[test]
    fn test_extract_calendar_id_from_pattern_wildcard_uuid() {
        assert_eq!(
            extract_calendar_id_from_pattern("calendar:*:entries:*"),
            None
        );
    }

    #[test]
    fn test_extract_calendar_id_from_pattern_non_calendar() {
        assert_eq!(extract_calendar_id_from_pattern("user:*"), None);
    }

    #[test]
    fn test_is_calendar_metadata_key() {
        let id = test_uuid();
        assert!(is_calendar_metadata_key(&format!("calendar:{}", id)));

        // Not metadata keys
        assert!(!is_calendar_metadata_key(&format!(
            "calendar:{}:entries:2024-01-01",
            id
        )));
        assert!(!is_calendar_metadata_key(&format!("calendar:{}:_keys", id)));
        assert!(!is_calendar_metadata_key("user:123"));
        assert!(!is_calendar_metadata_key("calendar:not-a-uuid"));
    }

    #[test]
    fn test_is_calendar_entry_key() {
        let id = test_uuid();
        assert!(is_calendar_entry_key(&format!(
            "calendar:{}:entries:2024-01-01:2024-01-31",
            id
        )));

        // Not entry keys
        assert!(!is_calendar_entry_key(&format!("calendar:{}", id)));
        assert!(!is_calendar_entry_key(&format!("calendar:{}:_keys", id)));
        assert!(!is_calendar_entry_key("user:123"));
        assert!(!is_calendar_entry_key("entry:456"));
    }
}
