//! DynamoDB attribute conversion functions.
//!
//! Pure functions for converting between DynamoDB AttributeValue maps and domain types.
//! These are testable in isolation without DynamoDB access.

use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use calendsync_core::calendar::{
    Calendar, CalendarEntry, CalendarMembership, CalendarRole, EntryKind, User,
};
use calendsync_core::storage::RepositoryError;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use super::keys;

// ============================================================================
// Entity type constants
// ============================================================================

pub const ENTITY_TYPE_USER: &str = "USER";
pub const ENTITY_TYPE_CALENDAR: &str = "CALENDAR";
pub const ENTITY_TYPE_ENTRY: &str = "ENTRY";
pub const ENTITY_TYPE_MEMBERSHIP: &str = "MEMBERSHIP";

// ============================================================================
// User conversions
// ============================================================================

/// Convert a User to DynamoDB item.
pub fn user_to_item(user: &User) -> HashMap<String, AttributeValue> {
    let mut item = HashMap::new();

    // Keys
    item.insert("PK".to_string(), AttributeValue::S(keys::user_pk(user.id)));
    item.insert("SK".to_string(), AttributeValue::S(keys::user_sk(user.id)));
    item.insert(
        "GSI2PK".to_string(),
        AttributeValue::S(keys::user_gsi2_pk(&user.email)),
    );
    item.insert(
        "GSI2SK".to_string(),
        AttributeValue::S(keys::user_gsi2_sk(user.id)),
    );

    // Entity type
    item.insert(
        "entityType".to_string(),
        AttributeValue::S(ENTITY_TYPE_USER.to_string()),
    );

    // Data
    item.insert("id".to_string(), AttributeValue::S(user.id.to_string()));
    item.insert("name".to_string(), AttributeValue::S(user.name.clone()));
    item.insert("email".to_string(), AttributeValue::S(user.email.clone()));
    item.insert(
        "createdAt".to_string(),
        AttributeValue::S(user.created_at.to_rfc3339()),
    );
    item.insert(
        "updatedAt".to_string(),
        AttributeValue::S(user.updated_at.to_rfc3339()),
    );

    item
}

/// Convert a DynamoDB item to User.
pub fn item_to_user(item: &HashMap<String, AttributeValue>) -> Result<User, RepositoryError> {
    Ok(User {
        id: get_uuid(item, "id")?,
        name: get_string(item, "name")?,
        email: get_string(item, "email")?,
        created_at: get_datetime(item, "createdAt")?,
        updated_at: get_datetime(item, "updatedAt")?,
    })
}

// ============================================================================
// Calendar conversions
// ============================================================================

/// Convert a Calendar to DynamoDB item.
pub fn calendar_to_item(calendar: &Calendar) -> HashMap<String, AttributeValue> {
    let mut item = HashMap::new();

    // Keys
    item.insert(
        "PK".to_string(),
        AttributeValue::S(keys::calendar_pk(calendar.id)),
    );
    item.insert(
        "SK".to_string(),
        AttributeValue::S(keys::calendar_sk(calendar.id)),
    );

    // Entity type
    item.insert(
        "entityType".to_string(),
        AttributeValue::S(ENTITY_TYPE_CALENDAR.to_string()),
    );

    // Data
    item.insert("id".to_string(), AttributeValue::S(calendar.id.to_string()));
    item.insert("name".to_string(), AttributeValue::S(calendar.name.clone()));
    item.insert(
        "color".to_string(),
        AttributeValue::S(calendar.color.clone()),
    );
    if let Some(desc) = &calendar.description {
        item.insert("description".to_string(), AttributeValue::S(desc.clone()));
    }
    item.insert(
        "createdAt".to_string(),
        AttributeValue::S(calendar.created_at.to_rfc3339()),
    );
    item.insert(
        "updatedAt".to_string(),
        AttributeValue::S(calendar.updated_at.to_rfc3339()),
    );

    item
}

/// Convert a DynamoDB item to Calendar.
pub fn item_to_calendar(
    item: &HashMap<String, AttributeValue>,
) -> Result<Calendar, RepositoryError> {
    Ok(Calendar {
        id: get_uuid(item, "id")?,
        name: get_string(item, "name")?,
        color: get_string(item, "color")?,
        description: get_optional_string(item, "description"),
        created_at: get_datetime(item, "createdAt")?,
        updated_at: get_datetime(item, "updatedAt")?,
    })
}

// ============================================================================
// Entry conversions
// ============================================================================

/// Convert a CalendarEntry to DynamoDB item.
pub fn entry_to_item(
    entry: &CalendarEntry,
) -> Result<HashMap<String, AttributeValue>, RepositoryError> {
    let mut item = HashMap::new();

    // Keys
    item.insert(
        "PK".to_string(),
        AttributeValue::S(keys::entry_pk(entry.id)),
    );
    item.insert(
        "SK".to_string(),
        AttributeValue::S(keys::entry_sk(entry.id)),
    );
    item.insert(
        "GSI1PK".to_string(),
        AttributeValue::S(keys::entry_gsi1_pk(entry.calendar_id)),
    );
    item.insert(
        "GSI1SK".to_string(),
        AttributeValue::S(keys::entry_gsi1_sk(entry.date, entry.id)),
    );

    // Entity type
    item.insert(
        "entityType".to_string(),
        AttributeValue::S(ENTITY_TYPE_ENTRY.to_string()),
    );

    // Data
    item.insert("id".to_string(), AttributeValue::S(entry.id.to_string()));
    item.insert(
        "calendarId".to_string(),
        AttributeValue::S(entry.calendar_id.to_string()),
    );
    item.insert("title".to_string(), AttributeValue::S(entry.title.clone()));
    item.insert(
        "date".to_string(),
        AttributeValue::S(entry.date.format("%Y-%m-%d").to_string()),
    );

    if let Some(desc) = &entry.description {
        item.insert("description".to_string(), AttributeValue::S(desc.clone()));
    }
    if let Some(loc) = &entry.location {
        item.insert("location".to_string(), AttributeValue::S(loc.clone()));
    }
    if let Some(color) = &entry.color {
        item.insert("color".to_string(), AttributeValue::S(color.clone()));
    }

    // Entry kind as JSON
    let kind_json = serde_json::to_string(&entry.kind)
        .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
    item.insert("kind".to_string(), AttributeValue::S(kind_json));

    item.insert(
        "createdAt".to_string(),
        AttributeValue::S(entry.created_at.to_rfc3339()),
    );
    item.insert(
        "updatedAt".to_string(),
        AttributeValue::S(entry.updated_at.to_rfc3339()),
    );

    Ok(item)
}

/// Convert a DynamoDB item to CalendarEntry.
pub fn item_to_entry(
    item: &HashMap<String, AttributeValue>,
) -> Result<CalendarEntry, RepositoryError> {
    let kind_json = get_string(item, "kind")?;
    let kind: EntryKind = serde_json::from_str(&kind_json)
        .map_err(|e| RepositoryError::Serialization(e.to_string()))?;

    Ok(CalendarEntry {
        id: get_uuid(item, "id")?,
        calendar_id: get_uuid(item, "calendarId")?,
        title: get_string(item, "title")?,
        description: get_optional_string(item, "description"),
        location: get_optional_string(item, "location"),
        kind,
        date: get_date(item, "date")?,
        color: get_optional_string(item, "color"),
        created_at: get_datetime(item, "createdAt")?,
        updated_at: get_datetime(item, "updatedAt")?,
    })
}

// ============================================================================
// Membership conversions
// ============================================================================

/// Convert a CalendarMembership to DynamoDB item.
pub fn membership_to_item(membership: &CalendarMembership) -> HashMap<String, AttributeValue> {
    let mut item = HashMap::new();

    // Keys
    item.insert(
        "PK".to_string(),
        AttributeValue::S(keys::membership_pk(membership.calendar_id)),
    );
    item.insert(
        "SK".to_string(),
        AttributeValue::S(keys::membership_sk(membership.user_id)),
    );
    item.insert(
        "GSI1PK".to_string(),
        AttributeValue::S(keys::membership_gsi1_pk(membership.user_id)),
    );
    item.insert(
        "GSI1SK".to_string(),
        AttributeValue::S(keys::membership_gsi1_sk(membership.calendar_id)),
    );

    // Entity type
    item.insert(
        "entityType".to_string(),
        AttributeValue::S(ENTITY_TYPE_MEMBERSHIP.to_string()),
    );

    // Data
    item.insert(
        "calendarId".to_string(),
        AttributeValue::S(membership.calendar_id.to_string()),
    );
    item.insert(
        "userId".to_string(),
        AttributeValue::S(membership.user_id.to_string()),
    );
    item.insert(
        "role".to_string(),
        AttributeValue::S(role_to_string(&membership.role).to_string()),
    );
    item.insert(
        "createdAt".to_string(),
        AttributeValue::S(membership.created_at.to_rfc3339()),
    );
    item.insert(
        "updatedAt".to_string(),
        AttributeValue::S(membership.updated_at.to_rfc3339()),
    );

    item
}

/// Convert a DynamoDB item to CalendarMembership.
pub fn item_to_membership(
    item: &HashMap<String, AttributeValue>,
) -> Result<CalendarMembership, RepositoryError> {
    Ok(CalendarMembership {
        calendar_id: get_uuid(item, "calendarId")?,
        user_id: get_uuid(item, "userId")?,
        role: parse_role(&get_string(item, "role")?)?,
        created_at: get_datetime(item, "createdAt")?,
        updated_at: get_datetime(item, "updatedAt")?,
    })
}

// ============================================================================
// Role conversions
// ============================================================================

/// Convert CalendarRole to string.
pub fn role_to_string(role: &CalendarRole) -> &'static str {
    match role {
        CalendarRole::Owner => "owner",
        CalendarRole::Writer => "writer",
        CalendarRole::Reader => "reader",
    }
}

/// Parse CalendarRole from string.
pub fn parse_role(s: &str) -> Result<CalendarRole, RepositoryError> {
    match s.to_lowercase().as_str() {
        "owner" => Ok(CalendarRole::Owner),
        "writer" => Ok(CalendarRole::Writer),
        "reader" => Ok(CalendarRole::Reader),
        _ => Err(RepositoryError::InvalidData(format!("Unknown role: {}", s))),
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Get a required string attribute.
fn get_string(
    item: &HashMap<String, AttributeValue>,
    key: &str,
) -> Result<String, RepositoryError> {
    item.get(key)
        .and_then(|v| v.as_s().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| RepositoryError::InvalidData(format!("Missing or invalid field: {}", key)))
}

/// Get an optional string attribute.
fn get_optional_string(item: &HashMap<String, AttributeValue>, key: &str) -> Option<String> {
    item.get(key)
        .and_then(|v| v.as_s().ok())
        .map(|s| s.to_string())
}

/// Get a required UUID attribute.
fn get_uuid(item: &HashMap<String, AttributeValue>, key: &str) -> Result<Uuid, RepositoryError> {
    let s = get_string(item, key)?;
    Uuid::parse_str(&s)
        .map_err(|e| RepositoryError::InvalidData(format!("Invalid UUID {}: {}", key, e)))
}

/// Get a required date attribute (YYYY-MM-DD format).
fn get_date(
    item: &HashMap<String, AttributeValue>,
    key: &str,
) -> Result<NaiveDate, RepositoryError> {
    let s = get_string(item, key)?;
    NaiveDate::parse_from_str(&s, "%Y-%m-%d")
        .map_err(|e| RepositoryError::InvalidData(format!("Invalid date {}: {}", key, e)))
}

/// Get a required datetime attribute (RFC 3339 format).
fn get_datetime(
    item: &HashMap<String, AttributeValue>,
    key: &str,
) -> Result<DateTime<Utc>, RepositoryError> {
    let s = get_string(item, key)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| RepositoryError::InvalidData(format!("Invalid datetime {}: {}", key, e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    fn sample_user() -> User {
        User {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            created_at: DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
        }
    }

    fn sample_calendar() -> Calendar {
        Calendar {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap(),
            name: "Personal".to_string(),
            color: "#3B82F6".to_string(),
            description: Some("My personal calendar".to_string()),
            created_at: DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
        }
    }

    fn sample_entry() -> CalendarEntry {
        CalendarEntry {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440003").unwrap(),
            calendar_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap(),
            title: "Standup Meeting".to_string(),
            description: None,
            location: Some("Zoom".to_string()),
            kind: EntryKind::Timed {
                start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            },
            date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            color: Some("#3B82F6".to_string()),
            created_at: DateTime::parse_from_rfc3339("2024-01-15T08:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2024-01-15T08:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        }
    }

    fn sample_membership() -> CalendarMembership {
        CalendarMembership {
            calendar_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap(),
            user_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
            role: CalendarRole::Owner,
            created_at: DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
        }
    }

    #[test]
    fn test_user_round_trip() {
        let user = sample_user();
        let item = user_to_item(&user);
        let parsed = item_to_user(&item).unwrap();

        assert_eq!(user.id, parsed.id);
        assert_eq!(user.name, parsed.name);
        assert_eq!(user.email, parsed.email);
    }

    #[test]
    fn test_user_item_has_correct_keys() {
        let user = sample_user();
        let item = user_to_item(&user);

        assert_eq!(
            item.get("PK").unwrap().as_s().unwrap(),
            "USER#550e8400-e29b-41d4-a716-446655440001"
        );
        assert_eq!(
            item.get("SK").unwrap().as_s().unwrap(),
            "USER#550e8400-e29b-41d4-a716-446655440001"
        );
        assert_eq!(
            item.get("GSI2PK").unwrap().as_s().unwrap(),
            "EMAIL#john@example.com"
        );
        assert_eq!(item.get("entityType").unwrap().as_s().unwrap(), "USER");
    }

    #[test]
    fn test_calendar_round_trip() {
        let calendar = sample_calendar();
        let item = calendar_to_item(&calendar);
        let parsed = item_to_calendar(&item).unwrap();

        assert_eq!(calendar.id, parsed.id);
        assert_eq!(calendar.name, parsed.name);
        assert_eq!(calendar.color, parsed.color);
        assert_eq!(calendar.description, parsed.description);
    }

    #[test]
    fn test_entry_round_trip() {
        let entry = sample_entry();
        let item = entry_to_item(&entry).unwrap();
        let parsed = item_to_entry(&item).unwrap();

        assert_eq!(entry.id, parsed.id);
        assert_eq!(entry.calendar_id, parsed.calendar_id);
        assert_eq!(entry.title, parsed.title);
        assert_eq!(entry.date, parsed.date);
        assert_eq!(entry.kind, parsed.kind);
    }

    #[test]
    fn test_entry_item_has_correct_gsi1_keys() {
        let entry = sample_entry();
        let item = entry_to_item(&entry).unwrap();

        assert_eq!(
            item.get("GSI1PK").unwrap().as_s().unwrap(),
            "CAL#550e8400-e29b-41d4-a716-446655440002"
        );
        assert!(item
            .get("GSI1SK")
            .unwrap()
            .as_s()
            .unwrap()
            .starts_with("ENTRY#2024-01-15#"));
    }

    #[test]
    fn test_membership_round_trip() {
        let membership = sample_membership();
        let item = membership_to_item(&membership);
        let parsed = item_to_membership(&item).unwrap();

        assert_eq!(membership.calendar_id, parsed.calendar_id);
        assert_eq!(membership.user_id, parsed.user_id);
        assert_eq!(membership.role, parsed.role);
    }

    #[test]
    fn test_role_conversions() {
        assert_eq!(role_to_string(&CalendarRole::Owner), "owner");
        assert_eq!(role_to_string(&CalendarRole::Writer), "writer");
        assert_eq!(role_to_string(&CalendarRole::Reader), "reader");

        assert_eq!(parse_role("owner").unwrap(), CalendarRole::Owner);
        assert_eq!(parse_role("OWNER").unwrap(), CalendarRole::Owner);
        assert_eq!(parse_role("Writer").unwrap(), CalendarRole::Writer);
        assert!(parse_role("invalid").is_err());
    }

    #[test]
    fn test_get_string_missing_field() {
        let item = HashMap::new();
        assert!(get_string(&item, "missing").is_err());
    }

    #[test]
    fn test_get_optional_string() {
        let mut item = HashMap::new();
        assert!(get_optional_string(&item, "missing").is_none());

        item.insert(
            "present".to_string(),
            AttributeValue::S("value".to_string()),
        );
        assert_eq!(
            get_optional_string(&item, "present"),
            Some("value".to_string())
        );
    }
}
