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
}
