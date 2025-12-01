use calendsync_core::calendar::{build_day_data, get_week_dates, CalendarEntry, DayData};
use chrono::{Duration, Local, NaiveDate, NaiveTime};
use uuid::Uuid;

/// Generates mock calendar entries for demonstration purposes.
/// Entries are spread across the week centered on the given date.
pub fn generate_mock_entries(calendar_id: Uuid, center_date: NaiveDate) -> Vec<CalendarEntry> {
    let mut entries = Vec::new();

    // Helper to create times
    let time = |h: u32, m: u32| NaiveTime::from_hms_opt(h, m, 0).unwrap();

    // Multi-day event: Team Retreat (3 days starting from center - 1)
    let retreat_start = center_date - Duration::days(1);
    let retreat_end = center_date + Duration::days(1);
    entries.push(
        CalendarEntry::multi_day(
            calendar_id,
            "Team Retreat",
            retreat_start,
            retreat_end,
            retreat_start,
        )
        .with_description("Annual team building event")
        .with_location("Mountain Lodge Resort")
        .with_color("#8B5CF6"), // Purple
    );

    // All-day events
    entries.push(
        CalendarEntry::all_day(
            calendar_id,
            "Sarah's Birthday",
            center_date + Duration::days(2),
        )
        .with_description("Don't forget the cake!")
        .with_color("#EC4899"), // Pink
    );

    entries.push(
        CalendarEntry::all_day(
            calendar_id,
            "Public Holiday",
            center_date - Duration::days(2),
        )
        .with_color("#10B981"), // Green
    );

    // Timed activities - spread across different days
    entries.push(
        CalendarEntry::timed(
            calendar_id,
            "Standup Meeting",
            center_date,
            time(9, 0),
            time(9, 30),
        )
        .with_location("Zoom")
        .with_color("#3B82F6"), // Blue
    );

    entries.push(
        CalendarEntry::timed(
            calendar_id,
            "Lunch with Alex",
            center_date,
            time(12, 30),
            time(13, 30),
        )
        .with_location("Cafe Bistro")
        .with_color("#F97316"), // Orange (accent)
    );

    entries.push(
        CalendarEntry::timed(
            calendar_id,
            "Product Review",
            center_date,
            time(15, 0),
            time(16, 0),
        )
        .with_description("Q4 roadmap discussion")
        .with_location("Conference Room A")
        .with_color("#3B82F6"), // Blue
    );

    entries.push(
        CalendarEntry::timed(
            calendar_id,
            "Gym Session",
            center_date,
            time(18, 0),
            time(19, 0),
        )
        .with_location("FitLife Gym")
        .with_color("#10B981"), // Green
    );

    entries.push(
        CalendarEntry::timed(
            calendar_id,
            "Team Sync",
            center_date + Duration::days(1),
            time(10, 0),
            time(11, 0),
        )
        .with_location("Meeting Room B")
        .with_color("#3B82F6"), // Blue
    );

    entries.push(
        CalendarEntry::timed(
            calendar_id,
            "Coffee with mentor",
            center_date + Duration::days(2),
            time(14, 0),
            time(15, 0),
        )
        .with_location("Coffee House")
        .with_color("#F97316"), // Orange
    );

    entries.push(
        CalendarEntry::timed(
            calendar_id,
            "Doctor Appointment",
            center_date - Duration::days(1),
            time(11, 0),
            time(11, 30),
        )
        .with_location("City Medical Center")
        .with_color("#EF4444"), // Red
    );

    // Tasks - various states
    entries.push(
        CalendarEntry::task(calendar_id, "Review PR #423", center_date, false)
            .with_description("Frontend refactoring changes"),
    );

    entries.push(
        CalendarEntry::task(calendar_id, "Send invoice", center_date, true)
            .with_description("Monthly billing for Project X"),
    );

    entries.push(
        CalendarEntry::task(
            calendar_id,
            "Call dentist",
            center_date + Duration::days(1),
            false,
        )
        .with_description("Schedule annual checkup"),
    );

    entries.push(CalendarEntry::task(
        calendar_id,
        "Update resume",
        center_date + Duration::days(3),
        false,
    ));

    entries.push(CalendarEntry::task(
        calendar_id,
        "Grocery shopping",
        center_date - Duration::days(1),
        true,
    ));

    entries
}

/// Builds calendar data for the week centered on the given date using a specific calendar.
pub fn build_demo_calendar_data_for_date_with_calendar(
    calendar_id: Uuid,
    center_date: NaiveDate,
) -> (NaiveDate, Vec<DayData>) {
    let today = Local::now().date_naive();
    let week_dates = get_week_dates(center_date);
    let entries = generate_mock_entries(calendar_id, center_date);
    let days = build_day_data(&week_dates, entries);

    (today, days)
}

/// Builds calendar data for the week centered on the given date.
/// Uses a temporary calendar ID for demo purposes.
pub fn build_demo_calendar_data_for_date(center_date: NaiveDate) -> (NaiveDate, Vec<DayData>) {
    let demo_calendar_id = Uuid::new_v4();
    build_demo_calendar_data_for_date_with_calendar(demo_calendar_id, center_date)
}

/// Builds calendar data for the week centered on today.
#[allow(dead_code)]
pub fn build_demo_calendar_data() -> (NaiveDate, Vec<DayData>) {
    let today = Local::now().date_naive();
    build_demo_calendar_data_for_date(today)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mock_entries() {
        let calendar_id = Uuid::new_v4();
        let center = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entries = generate_mock_entries(calendar_id, center);

        // Should have various entry types
        assert!(!entries.is_empty());

        let multi_day_count = entries.iter().filter(|e| e.kind.is_multi_day()).count();
        let all_day_count = entries.iter().filter(|e| e.kind.is_all_day()).count();
        let timed_count = entries.iter().filter(|e| e.kind.is_timed()).count();
        let task_count = entries.iter().filter(|e| e.kind.is_task()).count();

        assert!(multi_day_count >= 1, "Should have multi-day events");
        assert!(all_day_count >= 1, "Should have all-day events");
        assert!(timed_count >= 3, "Should have timed activities");
        assert!(task_count >= 3, "Should have tasks");

        // All entries should belong to the same calendar
        assert!(entries.iter().all(|e| e.calendar_id == calendar_id));
    }

    #[test]
    fn test_build_demo_calendar_data() {
        let (today, days) = build_demo_calendar_data();

        assert_eq!(days.len(), 7, "Should have 7 days");
        assert_eq!(days[3].date, today, "Center day should be today");
    }
}
