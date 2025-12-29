use chrono::NaiveDate;
use uuid::Uuid;

use super::error::{CalendarError, EntryError};
use super::types::{Calendar, CalendarEntry, EntryKind};

/// Filters entries by calendar ID.
pub fn filter_entries_by_calendar(
    entries: &[CalendarEntry],
    calendar_id: Uuid,
) -> Vec<&CalendarEntry> {
    entries
        .iter()
        .filter(|entry| entry.calendar_id == calendar_id)
        .collect()
}

/// Filters entries that overlap with a date range.
/// An entry overlaps if it starts before or on the end date AND ends on or after the start date.
pub fn filter_entries_by_date_range(
    entries: &[CalendarEntry],
    start: NaiveDate,
    end: NaiveDate,
) -> Vec<&CalendarEntry> {
    entries
        .iter()
        .filter(|entry| entry.start_date <= end && entry.end_date >= start)
        .collect()
}

/// Filters entries by calendar ID and date range.
/// Uses overlap logic: an entry is included if it overlaps with the date range.
pub fn filter_entries(
    entries: &[CalendarEntry],
    calendar_id: Option<Uuid>,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
) -> Vec<&CalendarEntry> {
    entries
        .iter()
        .filter(|entry| {
            calendar_id.is_none_or(|id| entry.calendar_id == id)
                && start.is_none_or(|s| entry.end_date >= s)
                && end.is_none_or(|e| entry.start_date <= e)
        })
        .collect()
}

/// Validates a calendar before creation or update.
pub fn validate_calendar(calendar: &Calendar) -> Result<(), CalendarError> {
    if calendar.name.trim().is_empty() {
        return Err(CalendarError::EmptyName);
    }
    if calendar.name.len() > 100 {
        return Err(CalendarError::NameTooLong);
    }
    if !is_valid_color(&calendar.color) {
        return Err(CalendarError::InvalidColor(calendar.color.clone()));
    }
    Ok(())
}

/// Validates a calendar entry before creation or update.
pub fn validate_entry(entry: &CalendarEntry) -> Result<(), EntryError> {
    if entry.title.trim().is_empty() {
        return Err(EntryError::EmptyTitle);
    }
    if entry.title.len() > 200 {
        return Err(EntryError::TitleTooLong);
    }

    // Validate date/time ranges for specific entry kinds
    match &entry.kind {
        EntryKind::MultiDay => {
            if entry.end_date < entry.start_date {
                return Err(EntryError::InvalidDateRange);
            }
        }
        EntryKind::Timed { start, end } => {
            if end <= start {
                return Err(EntryError::InvalidTimeRange);
            }
        }
        _ => {}
    }

    Ok(())
}

/// Checks if a color string is valid (hex color or CSS named color).
fn is_valid_color(color: &str) -> bool {
    if color.is_empty() {
        return false;
    }

    // Check hex color format (#RGB, #RRGGBB, #RRGGBBAA)
    if let Some(hex) = color.strip_prefix('#') {
        let valid_lengths = [3, 6, 8];
        return valid_lengths.contains(&hex.len()) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }

    // Allow common CSS color names
    let css_colors = [
        "red", "green", "blue", "yellow", "orange", "purple", "pink", "cyan", "magenta", "white",
        "black", "gray", "grey", "brown", "navy", "teal", "olive", "maroon", "lime", "aqua",
        "fuchsia", "silver",
    ];
    css_colors.contains(&color.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    fn test_calendar_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    }

    fn other_calendar_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap()
    }

    #[test]
    fn test_filter_entries_by_calendar() {
        let cal_id = test_calendar_id();
        let other_id = other_calendar_id();
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let entries = vec![
            CalendarEntry::all_day(cal_id, "Entry 1", date),
            CalendarEntry::all_day(other_id, "Entry 2", date),
            CalendarEntry::all_day(cal_id, "Entry 3", date),
        ];

        let filtered = filter_entries_by_calendar(&entries, cal_id);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|e| e.calendar_id == cal_id));
    }

    #[test]
    fn test_filter_entries_by_date_range() {
        let cal_id = test_calendar_id();
        let entries = vec![
            CalendarEntry::all_day(
                cal_id,
                "Before",
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            ),
            CalendarEntry::all_day(
                cal_id,
                "Start",
                NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(),
            ),
            CalendarEntry::all_day(
                cal_id,
                "Middle",
                NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            ),
            CalendarEntry::all_day(cal_id, "End", NaiveDate::from_ymd_opt(2024, 1, 20).unwrap()),
            CalendarEntry::all_day(
                cal_id,
                "After",
                NaiveDate::from_ymd_opt(2024, 1, 30).unwrap(),
            ),
        ];

        let start = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 20).unwrap();
        let filtered = filter_entries_by_date_range(&entries, start, end);

        assert_eq!(filtered.len(), 3);
        assert!(filtered.iter().any(|e| e.title == "Start"));
        assert!(filtered.iter().any(|e| e.title == "Middle"));
        assert!(filtered.iter().any(|e| e.title == "End"));
    }

    #[test]
    fn test_validate_calendar_success() {
        let calendar = Calendar::new("Work", "#3B82F6");
        assert!(validate_calendar(&calendar).is_ok());
    }

    #[test]
    fn test_validate_calendar_empty_name() {
        let calendar = Calendar::new("", "#3B82F6");
        assert_eq!(validate_calendar(&calendar), Err(CalendarError::EmptyName));

        let calendar = Calendar::new("   ", "#3B82F6");
        assert_eq!(validate_calendar(&calendar), Err(CalendarError::EmptyName));
    }

    #[test]
    fn test_validate_calendar_invalid_color() {
        let calendar = Calendar::new("Work", "not-a-color");
        assert!(matches!(
            validate_calendar(&calendar),
            Err(CalendarError::InvalidColor(_))
        ));
    }

    #[test]
    fn test_validate_entry_success() {
        let cal_id = test_calendar_id();
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let entry = CalendarEntry::all_day(cal_id, "Valid Entry", date);
        assert!(validate_entry(&entry).is_ok());
    }

    #[test]
    fn test_validate_entry_empty_title() {
        let cal_id = test_calendar_id();
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let entry = CalendarEntry::all_day(cal_id, "", date);
        assert_eq!(validate_entry(&entry), Err(EntryError::EmptyTitle));
    }

    #[test]
    fn test_validate_entry_invalid_date_range() {
        let cal_id = test_calendar_id();
        let start = NaiveDate::from_ymd_opt(2024, 1, 20).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(); // Before start!
        let entry = CalendarEntry::multi_day(cal_id, "Invalid", start, end);
        assert_eq!(validate_entry(&entry), Err(EntryError::InvalidDateRange));
    }

    #[test]
    fn test_validate_entry_invalid_time_range() {
        let cal_id = test_calendar_id();
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let start = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
        let end = NaiveTime::from_hms_opt(10, 0, 0).unwrap(); // Before start!
        let entry = CalendarEntry::timed(cal_id, "Invalid", date, start, end);
        assert_eq!(validate_entry(&entry), Err(EntryError::InvalidTimeRange));
    }

    #[test]
    fn test_is_valid_color() {
        // Valid hex colors
        assert!(is_valid_color("#FFF"));
        assert!(is_valid_color("#FFFFFF"));
        assert!(is_valid_color("#FFFFFFFF"));
        assert!(is_valid_color("#3B82F6"));

        // Valid CSS color names
        assert!(is_valid_color("red"));
        assert!(is_valid_color("Blue"));
        assert!(is_valid_color("GREEN"));

        // Invalid colors
        assert!(!is_valid_color(""));
        assert!(!is_valid_color("#GGG"));
        assert!(!is_valid_color("not-a-color"));
        assert!(!is_valid_color("#12345")); // Wrong length
    }
}
