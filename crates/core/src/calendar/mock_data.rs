//! Mock data generation for testing and seeding.
//!
//! This module contains pure functions for generating mock calendar entries.
//! These functions have no side effects and can be used in unit tests,
//! integration tests, and database seeding.

use super::types::{CalendarEntry, EntryKind};
use chrono::{Duration, NaiveDate, NaiveTime};
use uuid::Uuid;

/// Generate mock calendar entries spread around a center date.
///
/// Creates a realistic distribution of entries:
/// - ~15% multi-day events (conferences, vacations)
/// - ~20% all-day events (birthdays, holidays)
/// - ~45% timed activities (meetings, appointments)
/// - ~20% tasks (todos)
///
/// # Arguments
///
/// * `calendar_id` - The calendar to associate entries with
/// * `center_date` - The date to center the generated entries around
/// * `count` - Total number of entries to generate
///
/// # Example
///
/// ```
/// use calendsync_core::calendar::generate_seed_entries;
/// use chrono::NaiveDate;
/// use uuid::Uuid;
///
/// let calendar_id = Uuid::new_v4();
/// let center = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
/// let entries = generate_seed_entries(calendar_id, center, 20);
///
/// assert_eq!(entries.len(), 20);
/// ```
pub fn generate_seed_entries(
    calendar_id: Uuid,
    center_date: NaiveDate,
    count: u32,
) -> Vec<CalendarEntry> {
    let time = |h: u32, m: u32| NaiveTime::from_hms_opt(h, m, 0).unwrap();

    // Distribution: ~15% multi-day, ~20% all-day, ~45% timed, ~20% tasks
    let multi_day_count = (count as f32 * 0.15).ceil() as u32;
    let all_day_count = (count as f32 * 0.20).ceil() as u32;
    let timed_count = (count as f32 * 0.45).ceil() as u32;
    let task_count = count.saturating_sub(multi_day_count + all_day_count + timed_count);

    let mut entries = Vec::with_capacity(count as usize);

    // Multi-day events
    let multi_day_titles = [
        "Team Retreat",
        "Conference",
        "Vacation",
        "Training Workshop",
        "Hackathon",
    ];
    let multi_day_colors = ["#8B5CF6", "#EC4899", "#10B981", "#F59E0B", "#3B82F6"];
    for i in 0..multi_day_count {
        let start = center_date + Duration::days(i as i64 * 2 - 2);
        let end = start + Duration::days(2);
        let title = multi_day_titles[i as usize % multi_day_titles.len()];
        let color = multi_day_colors[i as usize % multi_day_colors.len()];
        entries.push(
            CalendarEntry::multi_day(calendar_id, title, start, end, start).with_color(color),
        );
    }

    // All-day events
    let all_day_titles = [
        "Birthday Party",
        "Public Holiday",
        "Company Anniversary",
        "Release Day",
        "Moving Day",
    ];
    let all_day_colors = ["#EC4899", "#10B981", "#F59E0B", "#3B82F6", "#8B5CF6"];
    for i in 0..all_day_count {
        let date = center_date + Duration::days(i as i64 - 1);
        let title = all_day_titles[i as usize % all_day_titles.len()];
        let color = all_day_colors[i as usize % all_day_colors.len()];
        entries.push(CalendarEntry::all_day(calendar_id, title, date).with_color(color));
    }

    // Timed activities
    let timed_titles = [
        "Standup Meeting",
        "Lunch with Team",
        "Product Review",
        "Gym Session",
        "Team Sync",
        "Coffee with Mentor",
        "Doctor Appointment",
        "Code Review",
        "Sprint Planning",
        "1:1 Meeting",
    ];
    let timed_colors = [
        "#3B82F6", "#F97316", "#3B82F6", "#10B981", "#3B82F6", "#F97316", "#EF4444", "#8B5CF6",
        "#3B82F6", "#F59E0B",
    ];
    for i in 0..timed_count {
        let date = center_date + Duration::days((i % 7) as i64 - 3);
        let start_hour = 8 + (i % 10);
        let title = timed_titles[i as usize % timed_titles.len()];
        let color = timed_colors[i as usize % timed_colors.len()];
        entries.push(
            CalendarEntry::timed(
                calendar_id,
                title,
                date,
                time(start_hour, 0),
                time(start_hour + 1, 0),
            )
            .with_color(color),
        );
    }

    // Tasks
    let task_titles = [
        "Review PR",
        "Send Invoice",
        "Update Documentation",
        "Grocery Shopping",
        "Book Flights",
        "Reply to Emails",
        "Clean Desk",
        "Update Resume",
    ];
    for i in 0..task_count {
        let date = center_date + Duration::days((i % 5) as i64 - 2);
        let title = task_titles[i as usize % task_titles.len()];
        let completed = i % 3 == 0; // ~33% completed
        entries.push(CalendarEntry::task(calendar_id, title, date, completed));
    }

    entries
}

/// Format entry kind for human-readable display.
///
/// Returns a lowercase string representation of the entry kind.
pub fn format_entry_kind(kind: &EntryKind) -> &'static str {
    match kind {
        EntryKind::MultiDay { .. } => "multi-day",
        EntryKind::AllDay => "all-day",
        EntryKind::Timed { .. } => "timed",
        EntryKind::Task { .. } => "task",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_seed_entries_count() {
        let calendar_id = Uuid::new_v4();
        let center = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        let entries = generate_seed_entries(calendar_id, center, 20);
        assert_eq!(entries.len(), 20);

        let entries = generate_seed_entries(calendar_id, center, 100);
        assert_eq!(entries.len(), 100);
    }

    #[test]
    fn test_generate_seed_entries_distribution() {
        let calendar_id = Uuid::new_v4();
        let center = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entries = generate_seed_entries(calendar_id, center, 100);

        let multi_day = entries
            .iter()
            .filter(|e| matches!(e.kind, EntryKind::MultiDay { .. }))
            .count();
        let all_day = entries
            .iter()
            .filter(|e| matches!(e.kind, EntryKind::AllDay))
            .count();
        let timed = entries
            .iter()
            .filter(|e| matches!(e.kind, EntryKind::Timed { .. }))
            .count();
        let tasks = entries
            .iter()
            .filter(|e| matches!(e.kind, EntryKind::Task { .. }))
            .count();

        // Check approximate distribution (allowing for rounding)
        assert!((10..=20).contains(&multi_day)); // ~15%
        assert!((15..=25).contains(&all_day)); // ~20%
        assert!((40..=50).contains(&timed)); // ~45%
        assert!((15..=25).contains(&tasks)); // ~20%
    }

    #[test]
    fn test_generate_seed_entries_calendar_id() {
        let calendar_id = Uuid::new_v4();
        let center = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entries = generate_seed_entries(calendar_id, center, 10);

        for entry in &entries {
            assert_eq!(entry.calendar_id, calendar_id);
        }
    }

    #[test]
    fn test_format_entry_kind() {
        assert_eq!(
            format_entry_kind(&EntryKind::MultiDay {
                start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                end: NaiveDate::from_ymd_opt(2024, 1, 3).unwrap()
            }),
            "multi-day"
        );
        assert_eq!(format_entry_kind(&EntryKind::AllDay), "all-day");
        assert_eq!(
            format_entry_kind(&EntryKind::Timed {
                start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(10, 0, 0).unwrap()
            }),
            "timed"
        );
        assert_eq!(
            format_entry_kind(&EntryKind::Task { completed: false }),
            "task"
        );
    }
}
