use calendsync_core::calendar::CalendarEntry;
use chrono::{Duration, NaiveDate, NaiveTime};
use uuid::Uuid;

/// Generates mock calendar entries for demonstration purposes.
/// Entries are spread across the week centered on the given date.
/// Includes 50+ entries on center_date to test vertical scrolling.
pub fn generate_mock_entries(calendar_id: Uuid, center_date: NaiveDate) -> Vec<CalendarEntry> {
    let mut entries = Vec::new();

    // Helper to create times
    let time = |h: u32, m: u32| NaiveTime::from_hms_opt(h, m, 0).unwrap();

    // Generate 50 entries for center_date to test vertical scrolling
    let scroll_test_titles = [
        "Morning Standup",
        "Code Review",
        "Design Sync",
        "Sprint Planning",
        "Customer Call",
        "Team Lunch",
        "1:1 with Manager",
        "Tech Talk",
        "Bug Triage",
        "Release Planning",
        "API Review",
        "Security Audit",
        "Performance Review",
        "Training Session",
        "Client Demo",
        "Retrospective",
        "Brainstorming",
        "Documentation",
        "Testing",
        "Deployment",
        "Monitoring Review",
        "Incident Postmortem",
        "Architecture Discussion",
        "Data Migration",
        "Feature Planning",
    ];

    let colors = [
        "#3B82F6", "#10B981", "#F97316", "#8B5CF6", "#EC4899", "#EF4444",
    ];

    for i in 0..50 {
        let title_idx = i % scroll_test_titles.len();
        let color_idx = i % colors.len();
        let hour = 6 + (i as u32 % 16); // 6 AM to 10 PM
        let minute = (i as u32 * 15) % 60;

        entries.push(
            CalendarEntry::timed(
                calendar_id,
                format!("{} #{}", scroll_test_titles[title_idx], i + 1),
                center_date,
                time(hour, minute),
                time(hour, (minute + 30) % 60),
            )
            .with_description(format!("Test entry {} for scroll testing", i + 1))
            .with_color(colors[color_idx]),
        );
    }

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
}
