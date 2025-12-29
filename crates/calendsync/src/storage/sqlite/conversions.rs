//! SQLite row conversion functions.
//!
//! Pure functions for converting between SQLite rows and domain types.
//! These are testable in isolation without database access.

use calendsync_core::calendar::{
    Calendar, CalendarEntry, CalendarMembership, CalendarRole, EntryKind, User,
};
use calendsync_core::storage::RepositoryError;
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::Row;
use uuid::Uuid;

// ============================================================================
// User conversions
// ============================================================================

/// Convert a SQLite row to a User.
///
/// Expected columns: id, name, email, created_at, updated_at
pub fn row_to_user(row: &Row) -> rusqlite::Result<User> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let email: String = row.get(2)?;
    let created_at: String = row.get(3)?;
    let updated_at: String = row.get(4)?;

    Ok(User {
        id: parse_uuid(&id)?,
        name,
        email,
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    })
}

// ============================================================================
// Calendar conversions
// ============================================================================

/// Convert a SQLite row to a Calendar.
///
/// Expected columns: id, name, color, description, created_at, updated_at
pub fn row_to_calendar(row: &Row) -> rusqlite::Result<Calendar> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let color: String = row.get(2)?;
    let description: Option<String> = row.get(3)?;
    let created_at: String = row.get(4)?;
    let updated_at: String = row.get(5)?;

    Ok(Calendar {
        id: parse_uuid(&id)?,
        name,
        color,
        description,
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    })
}

/// Convert a row to Calendar with role (from JOIN query).
///
/// Expected columns: id, name, color, description, created_at, updated_at, role
pub fn row_to_calendar_with_role(row: &Row) -> rusqlite::Result<(Calendar, CalendarRole)> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let color: String = row.get(2)?;
    let description: Option<String> = row.get(3)?;
    let created_at: String = row.get(4)?;
    let updated_at: String = row.get(5)?;
    let role_str: String = row.get(6)?;

    let calendar = Calendar {
        id: parse_uuid(&id)?,
        name,
        color,
        description,
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    };
    let role = parse_role(&role_str)?;

    Ok((calendar, role))
}

// ============================================================================
// Entry conversions
// ============================================================================

/// Convert a SQLite row to a CalendarEntry.
///
/// Expected columns: id, calendar_id, title, description, location, kind, start_date, end_date, color, created_at, updated_at
pub fn row_to_entry(row: &Row) -> rusqlite::Result<CalendarEntry> {
    let id: String = row.get(0)?;
    let calendar_id: String = row.get(1)?;
    let title: String = row.get(2)?;
    let description: Option<String> = row.get(3)?;
    let location: Option<String> = row.get(4)?;
    let kind_json: String = row.get(5)?;
    let start_date: String = row.get(6)?;
    let end_date: String = row.get(7)?;
    let color: Option<String> = row.get(8)?;
    let created_at: String = row.get(9)?;
    let updated_at: String = row.get(10)?;

    Ok(CalendarEntry {
        id: parse_uuid(&id)?,
        calendar_id: parse_uuid(&calendar_id)?,
        title,
        description,
        location,
        kind: json_to_entry_kind_internal(&kind_json)?,
        start_date: parse_date(&start_date)?,
        end_date: parse_date(&end_date)?,
        color,
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    })
}

/// Serialize EntryKind to JSON string.
pub fn entry_kind_to_json(kind: &EntryKind) -> Result<String, RepositoryError> {
    serde_json::to_string(kind).map_err(|e| RepositoryError::Serialization(e.to_string()))
}

/// Deserialize EntryKind from JSON string (for RepositoryError context).
pub fn json_to_entry_kind(json: &str) -> Result<EntryKind, RepositoryError> {
    serde_json::from_str(json).map_err(|e| RepositoryError::Serialization(e.to_string()))
}

/// Internal version that returns rusqlite::Result for use in row conversions.
fn json_to_entry_kind_internal(json: &str) -> rusqlite::Result<EntryKind> {
    serde_json::from_str(json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

// ============================================================================
// Membership conversions
// ============================================================================

/// Convert a SQLite row to a CalendarMembership.
///
/// Expected columns: calendar_id, user_id, role, created_at, updated_at
pub fn row_to_membership(row: &Row) -> rusqlite::Result<CalendarMembership> {
    let calendar_id: String = row.get(0)?;
    let user_id: String = row.get(1)?;
    let role_str: String = row.get(2)?;
    let created_at: String = row.get(3)?;
    let updated_at: String = row.get(4)?;

    Ok(CalendarMembership {
        calendar_id: parse_uuid(&calendar_id)?,
        user_id: parse_uuid(&user_id)?,
        role: parse_role(&role_str)?,
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    })
}

/// Convert a row to User with role (from JOIN query).
///
/// Expected columns: id, name, email, created_at, updated_at, role
pub fn row_to_user_with_role(row: &Row) -> rusqlite::Result<(User, CalendarRole)> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let email: String = row.get(2)?;
    let created_at: String = row.get(3)?;
    let updated_at: String = row.get(4)?;
    let role_str: String = row.get(5)?;

    let user = User {
        id: parse_uuid(&id)?,
        name,
        email,
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    };
    let role = parse_role(&role_str)?;

    Ok((user, role))
}

/// Serialize CalendarRole to string.
pub fn role_to_string(role: &CalendarRole) -> &'static str {
    match role {
        CalendarRole::Owner => "owner",
        CalendarRole::Writer => "writer",
        CalendarRole::Reader => "reader",
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Parse a UUID from string.
fn parse_uuid(s: &str) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

/// Parse a date from ISO 8601 string (YYYY-MM-DD).
fn parse_date(s: &str) -> rusqlite::Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

/// Parse a datetime from RFC 3339 string.
fn parse_datetime(s: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

/// Parse CalendarRole from string.
fn parse_role(s: &str) -> rusqlite::Result<CalendarRole> {
    match s.to_lowercase().as_str() {
        "owner" => Ok(CalendarRole::Owner),
        "writer" => Ok(CalendarRole::Writer),
        "reader" => Ok(CalendarRole::Reader),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown role: {}", s),
            )),
        )),
    }
}

/// Format a DateTime<Utc> for SQLite storage (RFC 3339).
pub fn format_datetime(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

/// Format a NaiveDate for SQLite storage (YYYY-MM-DD).
pub fn format_date(date: &NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    #[test]
    fn test_entry_kind_to_json_all_day() {
        let kind = EntryKind::AllDay;
        let json = entry_kind_to_json(&kind).unwrap();
        assert_eq!(json, r#""AllDay""#);
    }

    #[test]
    fn test_entry_kind_to_json_timed() {
        let kind = EntryKind::Timed {
            start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
        };
        let json = entry_kind_to_json(&kind).unwrap();
        assert!(json.contains("Timed"));
        assert!(json.contains("09:00:00"));
        assert!(json.contains("10:30:00"));
    }

    #[test]
    fn test_entry_kind_to_json_task() {
        let kind = EntryKind::Task { completed: true };
        let json = entry_kind_to_json(&kind).unwrap();
        assert!(json.contains("Task"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_entry_kind_to_json_multi_day() {
        let kind = EntryKind::MultiDay;
        let json = entry_kind_to_json(&kind).unwrap();
        assert_eq!(json, r#""MultiDay""#);
    }

    #[test]
    fn test_json_to_entry_kind_round_trip() {
        let kinds = vec![
            EntryKind::AllDay,
            EntryKind::Timed {
                start: NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
                end: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
            },
            EntryKind::Task { completed: false },
            EntryKind::MultiDay,
        ];

        for kind in kinds {
            let json = entry_kind_to_json(&kind).unwrap();
            let parsed = json_to_entry_kind(&json).unwrap();
            assert_eq!(kind, parsed);
        }
    }

    #[test]
    fn test_role_to_string() {
        assert_eq!(role_to_string(&CalendarRole::Owner), "owner");
        assert_eq!(role_to_string(&CalendarRole::Writer), "writer");
        assert_eq!(role_to_string(&CalendarRole::Reader), "reader");
    }

    #[test]
    fn test_parse_role() {
        assert!(parse_role("owner").is_ok());
        assert!(parse_role("OWNER").is_ok());
        assert!(parse_role("Writer").is_ok());
        assert!(parse_role("reader").is_ok());
    }

    #[test]
    fn test_parse_role_invalid() {
        assert!(parse_role("admin").is_err());
        assert!(parse_role("").is_err());
    }

    #[test]
    fn test_format_datetime() {
        let dt = DateTime::parse_from_rfc3339("2024-06-15T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let formatted = format_datetime(&dt);
        assert!(formatted.starts_with("2024-06-15"));
        assert!(formatted.contains("10:30:00"));
    }

    #[test]
    fn test_format_date() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        assert_eq!(format_date(&date), "2024-06-15");
    }

    #[test]
    fn test_parse_uuid_valid() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_uuid(uuid_str);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), uuid_str);
    }

    #[test]
    fn test_parse_uuid_invalid() {
        let result = parse_uuid("not-a-uuid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_date_valid() {
        let result = parse_date("2024-06-15");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()
        );
    }

    #[test]
    fn test_parse_date_invalid() {
        let result = parse_date("not-a-date");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_datetime_valid() {
        let result = parse_datetime("2024-06-15T10:30:00Z");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_datetime_invalid() {
        let result = parse_datetime("not-a-datetime");
        assert!(result.is_err());
    }
}
