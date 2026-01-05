//! DynamoDB key generation functions.
//!
//! Pure functions for generating partition and sort keys following the single-table design.
//! All functions are sync and have no side effects.

use chrono::NaiveDate;
use uuid::Uuid;

// ============================================================================
// Key prefixes
// ============================================================================

pub const USER_PREFIX: &str = "USER#";
pub const CALENDAR_PREFIX: &str = "CAL#";
pub const ENTRY_PREFIX: &str = "ENTRY#";
pub const MEMBER_PREFIX: &str = "MEMBER#";
pub const EMAIL_PREFIX: &str = "EMAIL#";
pub const PROVIDER_PREFIX: &str = "PROV#";

// ============================================================================
// User keys
// ============================================================================

/// Generate primary key for a User.
///
/// Pattern: `USER#<user_id>`
pub fn user_pk(user_id: Uuid) -> String {
    format!("{USER_PREFIX}{user_id}")
}

/// Generate sort key for a User.
///
/// Pattern: `USER#<user_id>` (same as PK for single-item queries)
pub fn user_sk(user_id: Uuid) -> String {
    format!("{USER_PREFIX}{user_id}")
}

/// Generate GSI2 partition key for User email lookup.
///
/// Pattern: `EMAIL#<email>`
pub fn user_gsi2_pk(email: &str) -> String {
    format!("{EMAIL_PREFIX}{email}")
}

/// Generate GSI2 sort key for User email lookup.
///
/// Pattern: `USER#<user_id>`
pub fn user_gsi2_sk(user_id: Uuid) -> String {
    format!("{USER_PREFIX}{user_id}")
}

/// Generate GSI3 partition key for User provider lookup.
///
/// Pattern: `PROV#<provider>#<provider_subject>`
pub fn user_gsi3_pk(provider: &str, provider_subject: &str) -> String {
    format!("{PROVIDER_PREFIX}{}#{}", provider, provider_subject)
}

/// Generate GSI3 sort key for User provider lookup.
///
/// Pattern: `USER#<user_id>`
pub fn user_gsi3_sk(user_id: Uuid) -> String {
    format!("{USER_PREFIX}{user_id}")
}

// ============================================================================
// Calendar keys
// ============================================================================

/// Generate primary key for a Calendar.
///
/// Pattern: `CAL#<calendar_id>`
pub fn calendar_pk(calendar_id: Uuid) -> String {
    format!("{CALENDAR_PREFIX}{calendar_id}")
}

/// Generate sort key for a Calendar.
///
/// Pattern: `CAL#<calendar_id>` (same as PK for single-item queries)
pub fn calendar_sk(calendar_id: Uuid) -> String {
    format!("{CALENDAR_PREFIX}{calendar_id}")
}

// ============================================================================
// Entry keys
// ============================================================================

/// Generate primary key for an Entry.
///
/// Pattern: `ENTRY#<entry_id>`
pub fn entry_pk(entry_id: Uuid) -> String {
    format!("{ENTRY_PREFIX}{entry_id}")
}

/// Generate sort key for an Entry.
///
/// Pattern: `ENTRY#<entry_id>` (same as PK for single-item queries)
pub fn entry_sk(entry_id: Uuid) -> String {
    format!("{ENTRY_PREFIX}{entry_id}")
}

/// Generate GSI1 partition key for Entry (calendar lookup).
///
/// Pattern: `CAL#<calendar_id>`
pub fn entry_gsi1_pk(calendar_id: Uuid) -> String {
    format!("{CALENDAR_PREFIX}{calendar_id}")
}

/// Generate GSI1 sort key for Entry (date-sorted lookup).
///
/// Pattern: `ENTRY#<start_date>#<entry_id>`
///
/// The start_date is in ISO 8601 format (YYYY-MM-DD) for lexicographic sorting.
pub fn entry_gsi1_sk(start_date: NaiveDate, entry_id: Uuid) -> String {
    format!("{ENTRY_PREFIX}{}#{entry_id}", start_date.format("%Y-%m-%d"))
}

/// Generate the start bound for a date range query on GSI1SK.
///
/// Pattern: `ENTRY#<date>#`
///
/// The trailing `#` ensures we match entries from the start of the day.
pub fn entry_gsi1_sk_start(date: NaiveDate) -> String {
    format!("{ENTRY_PREFIX}{}#", date.format("%Y-%m-%d"))
}

/// Generate the end bound for a date range query on GSI1SK.
///
/// Pattern: `ENTRY#<date>#~`
///
/// The `~` character (ASCII 126) is higher than any UUID character,
/// ensuring all entries on the end date are included.
pub fn entry_gsi1_sk_end(date: NaiveDate) -> String {
    format!("{ENTRY_PREFIX}{}#~", date.format("%Y-%m-%d"))
}

/// Generate the maximum sort key for overlap queries.
/// Used to find all entries starting on or before a given date.
///
/// Pattern: `ENTRY#<date>#~`
pub fn entry_gsi1_sk_max(date: NaiveDate) -> String {
    format!("{ENTRY_PREFIX}{}#~", date.format("%Y-%m-%d"))
}

// ============================================================================
// Membership keys
// ============================================================================

/// Generate primary key for a Membership.
///
/// Pattern: `CAL#<calendar_id>`
pub fn membership_pk(calendar_id: Uuid) -> String {
    format!("{CALENDAR_PREFIX}{calendar_id}")
}

/// Generate sort key for a Membership.
///
/// Pattern: `MEMBER#<user_id>`
pub fn membership_sk(user_id: Uuid) -> String {
    format!("{MEMBER_PREFIX}{user_id}")
}

/// Generate GSI1 partition key for Membership (user's calendars lookup).
///
/// Pattern: `USER#<user_id>`
pub fn membership_gsi1_pk(user_id: Uuid) -> String {
    format!("{USER_PREFIX}{user_id}")
}

/// Generate GSI1 sort key for Membership.
///
/// Pattern: `CAL#<calendar_id>`
pub fn membership_gsi1_sk(calendar_id: Uuid) -> String {
    format!("{CALENDAR_PREFIX}{calendar_id}")
}

/// Generate the sort key prefix for querying all members of a calendar.
///
/// Pattern: `MEMBER#`
pub fn membership_sk_prefix() -> &'static str {
    MEMBER_PREFIX
}

/// Generate the GSI1SK prefix for querying all calendars of a user.
///
/// Pattern: `CAL#`
pub fn calendar_gsi1_sk_prefix() -> &'static str {
    CALENDAR_PREFIX
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_user_pk() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
        assert_eq!(user_pk(id), "USER#550e8400-e29b-41d4-a716-446655440001");
    }

    #[test]
    fn test_user_gsi2_pk() {
        assert_eq!(user_gsi2_pk("john@example.com"), "EMAIL#john@example.com");
    }

    #[test]
    fn test_user_gsi3_pk() {
        assert_eq!(user_gsi3_pk("google", "123456789"), "PROV#google#123456789");
    }

    #[test]
    fn test_user_gsi3_sk() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
        assert_eq!(
            user_gsi3_sk(id),
            "USER#550e8400-e29b-41d4-a716-446655440001"
        );
    }

    #[test]
    fn test_calendar_pk() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap();
        assert_eq!(calendar_pk(id), "CAL#550e8400-e29b-41d4-a716-446655440002");
    }

    #[test]
    fn test_entry_pk() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440003").unwrap();
        assert_eq!(entry_pk(id), "ENTRY#550e8400-e29b-41d4-a716-446655440003");
    }

    #[test]
    fn test_entry_gsi1_sk() {
        let start_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440003").unwrap();
        assert_eq!(
            entry_gsi1_sk(start_date, id),
            "ENTRY#2024-06-15#550e8400-e29b-41d4-a716-446655440003"
        );
    }

    #[test]
    fn test_entry_gsi1_sk_range_bounds() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert_eq!(entry_gsi1_sk_start(date), "ENTRY#2024-01-15#");
        assert_eq!(entry_gsi1_sk_end(date), "ENTRY#2024-01-15#~");
    }

    #[test]
    fn test_entry_gsi1_sk_max() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 20).unwrap();
        assert_eq!(entry_gsi1_sk_max(date), "ENTRY#2024-03-20#~");
    }

    #[test]
    fn test_membership_pk() {
        let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap();
        assert_eq!(
            membership_pk(calendar_id),
            "CAL#550e8400-e29b-41d4-a716-446655440002"
        );
    }

    #[test]
    fn test_membership_sk() {
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
        assert_eq!(
            membership_sk(user_id),
            "MEMBER#550e8400-e29b-41d4-a716-446655440001"
        );
    }

    #[test]
    fn test_membership_gsi1_keys() {
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
        let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap();

        assert_eq!(
            membership_gsi1_pk(user_id),
            "USER#550e8400-e29b-41d4-a716-446655440001"
        );
        assert_eq!(
            membership_gsi1_sk(calendar_id),
            "CAL#550e8400-e29b-41d4-a716-446655440002"
        );
    }

    #[test]
    fn test_prefixes() {
        assert_eq!(membership_sk_prefix(), "MEMBER#");
        assert_eq!(calendar_gsi1_sk_prefix(), "CAL#");
    }
}
