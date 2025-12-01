use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate};

use super::types::{CalendarEntry, DayData};

/// Sorts entries by hierarchy: MultiDay -> AllDay -> Timed -> Task.
/// Within timed entries, sorts by start time.
pub fn sort_entries_by_hierarchy(entries: &mut [CalendarEntry]) {
    entries.sort_by(|a, b| {
        let priority_cmp = a.kind.sort_priority().cmp(&b.kind.sort_priority());
        if priority_cmp != std::cmp::Ordering::Equal {
            return priority_cmp;
        }

        // Within the same priority, sort timed entries by start time
        match (a.kind.start_time(), b.kind.start_time()) {
            (Some(a_time), Some(b_time)) => a_time.cmp(&b_time),
            _ => std::cmp::Ordering::Equal,
        }
    });
}

/// Groups entries by their display date.
pub fn group_entries_by_date(entries: &[CalendarEntry]) -> HashMap<NaiveDate, Vec<&CalendarEntry>> {
    let mut grouped: HashMap<NaiveDate, Vec<&CalendarEntry>> = HashMap::new();

    for entry in entries {
        grouped.entry(entry.date).or_default().push(entry);
    }

    grouped
}

/// Returns 7 dates centered on the given date (3 before, center, 3 after).
pub fn get_week_dates(center: NaiveDate) -> Vec<NaiveDate> {
    (-3..=3)
        .map(|offset| center + Duration::days(offset))
        .collect()
}

/// Returns the dates for the week containing the given date (Monday to Sunday).
pub fn get_calendar_week(date: NaiveDate) -> Vec<NaiveDate> {
    let weekday = date.weekday().num_days_from_monday() as i64;
    let monday = date - Duration::days(weekday);

    (0..7)
        .map(|offset| monday + Duration::days(offset))
        .collect()
}

/// Expands multi-day events into separate entries for each day they span.
pub fn expand_multi_day_entries(entries: Vec<CalendarEntry>) -> Vec<CalendarEntry> {
    let mut expanded = Vec::new();

    for entry in entries {
        match &entry.kind {
            super::types::EntryKind::MultiDay { start, end } => {
                let mut current = *start;
                while current <= *end {
                    let mut day_entry = entry.clone();
                    day_entry.date = current;
                    expanded.push(day_entry);
                    current += Duration::days(1);
                }
            }
            _ => expanded.push(entry),
        }
    }

    expanded
}

/// Builds DayData for a range of dates from the given entries.
/// Entries are sorted by hierarchy within each day.
pub fn build_day_data(dates: &[NaiveDate], entries: Vec<CalendarEntry>) -> Vec<DayData> {
    let expanded = expand_multi_day_entries(entries);
    let grouped = group_entries_by_date(&expanded);

    dates
        .iter()
        .map(|date| {
            let mut day_entries: Vec<CalendarEntry> = grouped
                .get(date)
                .map(|refs| refs.iter().map(|e| (*e).clone()).collect())
                .unwrap_or_default();

            sort_entries_by_hierarchy(&mut day_entries);

            DayData::new(*date, day_entries)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::types::CalendarEntry;
    use chrono::NaiveTime;
    use uuid::Uuid;

    fn test_calendar_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    }

    fn make_date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn make_time(hour: u32, min: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, min, 0).unwrap()
    }

    #[test]
    fn test_sort_entries_by_hierarchy() {
        let cal_id = test_calendar_id();
        let date = make_date(2024, 1, 15);
        let mut entries = vec![
            CalendarEntry::task(cal_id, "Task", date, false),
            CalendarEntry::timed(cal_id, "Meeting", date, make_time(14, 0), make_time(15, 0)),
            CalendarEntry::all_day(cal_id, "Birthday", date),
            CalendarEntry::timed(cal_id, "Standup", date, make_time(9, 0), make_time(9, 30)),
            CalendarEntry::multi_day(cal_id, "Retreat", date, date + Duration::days(2), date),
        ];

        sort_entries_by_hierarchy(&mut entries);

        assert!(entries[0].kind.is_multi_day());
        assert!(entries[1].kind.is_all_day());
        assert!(entries[2].kind.is_timed());
        assert_eq!(entries[2].title, "Standup"); // 9:00 before 14:00
        assert!(entries[3].kind.is_timed());
        assert_eq!(entries[3].title, "Meeting");
        assert!(entries[4].kind.is_task());
    }

    #[test]
    fn test_get_week_dates() {
        let center = make_date(2024, 1, 15); // Monday
        let dates = get_week_dates(center);

        assert_eq!(dates.len(), 7);
        assert_eq!(dates[0], make_date(2024, 1, 12)); // 3 days before
        assert_eq!(dates[3], center); // Center
        assert_eq!(dates[6], make_date(2024, 1, 18)); // 3 days after
    }

    #[test]
    fn test_get_calendar_week() {
        let date = make_date(2024, 1, 17); // Wednesday
        let week = get_calendar_week(date);

        assert_eq!(week.len(), 7);
        assert_eq!(week[0], make_date(2024, 1, 15)); // Monday
        assert_eq!(week[6], make_date(2024, 1, 21)); // Sunday
    }

    #[test]
    fn test_expand_multi_day_entries() {
        let cal_id = test_calendar_id();
        let start = make_date(2024, 1, 15);
        let end = make_date(2024, 1, 17);
        let entries = vec![
            CalendarEntry::multi_day(cal_id, "Retreat", start, end, start),
            CalendarEntry::all_day(cal_id, "Single", start),
        ];

        let expanded = expand_multi_day_entries(entries);

        assert_eq!(expanded.len(), 4); // 3 days for retreat + 1 single
        assert_eq!(expanded.iter().filter(|e| e.title == "Retreat").count(), 3);
    }

    #[test]
    fn test_group_entries_by_date() {
        let cal_id = test_calendar_id();
        let date1 = make_date(2024, 1, 15);
        let date2 = make_date(2024, 1, 16);
        let entries = vec![
            CalendarEntry::all_day(cal_id, "Event 1", date1),
            CalendarEntry::all_day(cal_id, "Event 2", date1),
            CalendarEntry::all_day(cal_id, "Event 3", date2),
        ];

        let grouped = group_entries_by_date(&entries);

        assert_eq!(grouped.get(&date1).unwrap().len(), 2);
        assert_eq!(grouped.get(&date2).unwrap().len(), 1);
    }

    #[test]
    fn test_build_day_data() {
        let cal_id = test_calendar_id();
        let date1 = make_date(2024, 1, 15);
        let date2 = make_date(2024, 1, 16);
        let dates = vec![date1, date2];

        let entries = vec![
            CalendarEntry::task(cal_id, "Task", date1, false),
            CalendarEntry::all_day(cal_id, "Birthday", date1),
            CalendarEntry::all_day(cal_id, "Meeting", date2),
        ];

        let day_data = build_day_data(&dates, entries);

        assert_eq!(day_data.len(), 2);

        // First day should have entries sorted by hierarchy
        assert_eq!(day_data[0].entries.len(), 2);
        assert!(day_data[0].entries[0].kind.is_all_day()); // AllDay before Task
        assert!(day_data[0].entries[1].kind.is_task());

        // Second day
        assert_eq!(day_data[1].entries.len(), 1);
    }
}
