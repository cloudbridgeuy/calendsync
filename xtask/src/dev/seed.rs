//! Seeding types and functions for calendar data generation.
//!
//! This module provides:
//! - **Pure functions**: Data generation and conversion (`convert_entry_to_seed`,
//!   `generate_entries_for_seeding`)
//! - **Types**: `SeedEntry` for HTTP form submission
//!
//! The standalone `cargo xtask seed` command uses these to create entries
//! with authentication support.

use chrono::{NaiveDate, NaiveTime};
use serde::Serialize;
use uuid::Uuid;

use calendsync_core::calendar::{generate_seed_entries, CalendarEntry, EntryKind};

// ============================================================================
// Types
// ============================================================================

/// Entry data for seeding via HTTP.
/// Field names match the server's CreateEntry form expectations.
#[derive(Debug, Serialize)]
pub struct SeedEntry {
    pub calendar_id: Uuid,
    pub title: String,
    pub start_date: NaiveDate,
    pub entry_type: String, // "all_day", "timed", "task", "multi_day"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<NaiveTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<NaiveTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NaiveDate>,
}

// ============================================================================
// Pure Functions (Functional Core)
// ============================================================================

/// Converts an `EntryKind` to the form string expected by the server.
pub fn entry_type_string(kind: &EntryKind) -> &'static str {
    match kind {
        EntryKind::AllDay => "all_day",
        EntryKind::Timed { .. } => "timed",
        EntryKind::Task { .. } => "task",
        EntryKind::MultiDay => "multi_day",
    }
}

/// Converts a `CalendarEntry` to a `SeedEntry` for HTTP submission.
pub fn convert_entry_to_seed(entry: &CalendarEntry) -> SeedEntry {
    let entry_type = entry_type_string(&entry.kind).to_string();

    // Extract time fields from Timed variant
    let (start_time, end_time) = match &entry.kind {
        EntryKind::Timed { start, end } => (Some(*start), Some(*end)),
        _ => (None, None),
    };

    // Extract end_date for MultiDay entries (now stored in entry.end_date)
    let end_date = match &entry.kind {
        EntryKind::MultiDay => Some(entry.end_date),
        _ => None,
    };

    SeedEntry {
        calendar_id: entry.calendar_id,
        title: entry.title.clone(),
        start_date: entry.start_date,
        entry_type,
        description: entry.description.clone(),
        location: entry.location.clone(),
        color: entry.color.clone(),
        start_time,
        end_time,
        end_date,
    }
}

/// Generates seed entries for HTTP submission.
///
/// Uses `calendsync_core::calendar::generate_seed_entries` for data generation,
/// then converts each entry to the `SeedEntry` format.
pub fn generate_entries_for_seeding(
    calendar_id: Uuid,
    center_date: NaiveDate,
    count: u32,
) -> Vec<SeedEntry> {
    let entries = generate_seed_entries(calendar_id, center_date, count);
    entries.iter().map(convert_entry_to_seed).collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_entry_type_string() {
        assert_eq!(entry_type_string(&EntryKind::AllDay), "all_day");
        assert_eq!(
            entry_type_string(&EntryKind::Timed {
                start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            }),
            "timed"
        );
        assert_eq!(
            entry_type_string(&EntryKind::Task { completed: false }),
            "task"
        );
        assert_eq!(entry_type_string(&EntryKind::MultiDay), "multi_day");
    }

    #[test]
    fn test_convert_entry_to_seed_all_day() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = CalendarEntry::all_day(calendar_id, "Birthday", date)
            .with_description("John's birthday")
            .with_color("#EC4899");

        let seed = convert_entry_to_seed(&entry);

        assert_eq!(seed.calendar_id, calendar_id);
        assert_eq!(seed.title, "Birthday");
        assert_eq!(seed.start_date, date);
        assert_eq!(seed.entry_type, "all_day");
        assert_eq!(seed.description, Some("John's birthday".to_string()));
        assert_eq!(seed.color, Some("#EC4899".to_string()));
        assert!(seed.start_time.is_none());
        assert!(seed.end_time.is_none());
        assert!(seed.end_date.is_none());
    }

    #[test]
    fn test_convert_entry_to_seed_timed() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let start = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        let end = NaiveTime::from_hms_opt(11, 30, 0).unwrap();
        let entry = CalendarEntry::timed(calendar_id, "Meeting", date, start, end);

        let seed = convert_entry_to_seed(&entry);

        assert_eq!(seed.entry_type, "timed");
        assert_eq!(seed.start_time, Some(start));
        assert_eq!(seed.end_time, Some(end));
        assert!(seed.end_date.is_none());
    }

    #[test]
    fn test_convert_entry_to_seed_multi_day() {
        let calendar_id = Uuid::new_v4();
        let start = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 18).unwrap();
        let entry = CalendarEntry::multi_day(calendar_id, "Vacation", start, end);

        let seed = convert_entry_to_seed(&entry);

        assert_eq!(seed.entry_type, "multi_day");
        assert_eq!(seed.start_date, start);
        assert_eq!(seed.end_date, Some(end));
        assert!(seed.start_time.is_none());
        assert!(seed.end_time.is_none());
    }

    #[test]
    fn test_convert_entry_to_seed_task() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = CalendarEntry::task(calendar_id, "Review PR", date, false);

        let seed = convert_entry_to_seed(&entry);

        assert_eq!(seed.entry_type, "task");
        assert!(seed.start_time.is_none());
        assert!(seed.end_time.is_none());
        assert!(seed.end_date.is_none());
    }

    #[test]
    fn test_generate_entries_for_seeding() {
        let calendar_id = Uuid::new_v4();
        let center = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        let entries = generate_entries_for_seeding(calendar_id, center, 10);

        assert_eq!(entries.len(), 10);
        for entry in &entries {
            assert_eq!(entry.calendar_id, calendar_id);
            assert!(!entry.title.is_empty());
            // Verify entry_type is valid
            assert!(
                entry.entry_type == "all_day"
                    || entry.entry_type == "timed"
                    || entry.entry_type == "task"
                    || entry.entry_type == "multi_day"
            );
        }
    }
}
