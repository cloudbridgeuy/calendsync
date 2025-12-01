use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A named calendar that contains entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calendar {
    pub id: Uuid,
    pub name: String,
    /// Default color for entries in this calendar (CSS color value).
    pub color: String,
    pub description: Option<String>,
}

impl Calendar {
    /// Creates a new calendar with the given name and color.
    pub fn new(name: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            color: color.into(),
            description: None,
        }
    }

    /// Sets the description for this calendar.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets a specific ID for this calendar (useful for testing).
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }
}

/// The kind of calendar entry, determining its display behavior and hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    /// An event spanning multiple days.
    MultiDay { start: NaiveDate, end: NaiveDate },
    /// An all-day event (no specific time).
    AllDay,
    /// A timed activity with start and end times.
    Timed { start: NaiveTime, end: NaiveTime },
    /// A task that can be marked as completed.
    Task { completed: bool },
}

impl EntryKind {
    /// Returns the sort priority for this entry kind.
    /// Lower values appear first in the hierarchy.
    pub fn sort_priority(&self) -> u8 {
        match self {
            EntryKind::MultiDay { .. } => 0,
            EntryKind::AllDay => 1,
            EntryKind::Timed { .. } => 2,
            EntryKind::Task { .. } => 3,
        }
    }

    /// Returns true if this is a multi-day event.
    pub fn is_multi_day(&self) -> bool {
        matches!(self, EntryKind::MultiDay { .. })
    }

    /// Returns true if this is an all-day event.
    pub fn is_all_day(&self) -> bool {
        matches!(self, EntryKind::AllDay)
    }

    /// Returns true if this is a timed activity.
    pub fn is_timed(&self) -> bool {
        matches!(self, EntryKind::Timed { .. })
    }

    /// Returns true if this is a task.
    pub fn is_task(&self) -> bool {
        matches!(self, EntryKind::Task { .. })
    }

    /// Returns the start time if this is a timed entry.
    pub fn start_time(&self) -> Option<NaiveTime> {
        match self {
            EntryKind::Timed { start, .. } => Some(*start),
            _ => None,
        }
    }

    /// Returns the end time if this is a timed entry.
    pub fn end_time(&self) -> Option<NaiveTime> {
        match self {
            EntryKind::Timed { end, .. } => Some(*end),
            _ => None,
        }
    }

    /// Returns the CSS class name for this entry kind.
    pub fn css_class(&self) -> &'static str {
        match self {
            EntryKind::MultiDay { .. } => "multi-day",
            EntryKind::AllDay => "all-day",
            EntryKind::Timed { .. } => "timed",
            EntryKind::Task { .. } => "task",
        }
    }

    /// Returns true if this is a completed task.
    pub fn is_completed(&self) -> bool {
        matches!(self, EntryKind::Task { completed: true })
    }

    /// Returns the start date for multi-day events.
    pub fn multi_day_start(&self) -> Option<NaiveDate> {
        match self {
            EntryKind::MultiDay { start, .. } => Some(*start),
            _ => None,
        }
    }

    /// Returns the end date for multi-day events.
    pub fn multi_day_end(&self) -> Option<NaiveDate> {
        match self {
            EntryKind::MultiDay { end, .. } => Some(*end),
            _ => None,
        }
    }
}

/// A calendar entry representing an event, activity, or task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEntry {
    pub id: Uuid,
    /// The calendar this entry belongs to.
    pub calendar_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub kind: EntryKind,
    /// The display date for this entry.
    /// For multi-day events, entries are duplicated for each day they span.
    pub date: NaiveDate,
    /// Optional accent color for the entry tile (CSS color value).
    pub color: Option<String>,
}

impl CalendarEntry {
    /// Creates a new multi-day event.
    pub fn multi_day(
        calendar_id: Uuid,
        title: impl Into<String>,
        start: NaiveDate,
        end: NaiveDate,
        display_date: NaiveDate,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            calendar_id,
            title: title.into(),
            description: None,
            location: None,
            kind: EntryKind::MultiDay { start, end },
            date: display_date,
            color: None,
        }
    }

    /// Creates a new all-day event.
    pub fn all_day(calendar_id: Uuid, title: impl Into<String>, date: NaiveDate) -> Self {
        Self {
            id: Uuid::new_v4(),
            calendar_id,
            title: title.into(),
            description: None,
            location: None,
            kind: EntryKind::AllDay,
            date,
            color: None,
        }
    }

    /// Creates a new timed activity.
    pub fn timed(
        calendar_id: Uuid,
        title: impl Into<String>,
        date: NaiveDate,
        start: NaiveTime,
        end: NaiveTime,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            calendar_id,
            title: title.into(),
            description: None,
            location: None,
            kind: EntryKind::Timed { start, end },
            date,
            color: None,
        }
    }

    /// Creates a new task.
    pub fn task(
        calendar_id: Uuid,
        title: impl Into<String>,
        date: NaiveDate,
        completed: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            calendar_id,
            title: title.into(),
            description: None,
            location: None,
            kind: EntryKind::Task { completed },
            date,
            color: None,
        }
    }

    /// Sets the calendar ID for this entry.
    pub fn with_calendar_id(mut self, calendar_id: Uuid) -> Self {
        self.calendar_id = calendar_id;
        self
    }

    /// Sets the description for this entry.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the location for this entry.
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Sets the accent color for this entry.
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets a specific ID for this entry (useful for testing).
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }
}

/// Data for a single day in the calendar view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayData {
    pub date: NaiveDate,
    pub entries: Vec<CalendarEntry>,
}

impl DayData {
    /// Creates a new DayData with the given date and entries.
    pub fn new(date: NaiveDate, entries: Vec<CalendarEntry>) -> Self {
        Self { date, entries }
    }

    /// Creates an empty DayData for the given date.
    pub fn empty(date: NaiveDate) -> Self {
        Self {
            date,
            entries: Vec::new(),
        }
    }

    /// Returns true if this day has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the number of entries for this day.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_entry_kind_sort_priority() {
        let multi_day = EntryKind::MultiDay {
            start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
        };
        let all_day = EntryKind::AllDay;
        let timed = EntryKind::Timed {
            start: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
        };
        let task = EntryKind::Task { completed: false };

        assert!(multi_day.sort_priority() < all_day.sort_priority());
        assert!(all_day.sort_priority() < timed.sort_priority());
        assert!(timed.sort_priority() < task.sort_priority());
    }

    #[test]
    fn test_calendar_builder() {
        let calendar = Calendar::new("Work", "#3B82F6").with_description("Work calendar");

        assert_eq!(calendar.name, "Work");
        assert_eq!(calendar.color, "#3B82F6");
        assert_eq!(calendar.description, Some("Work calendar".to_string()));
    }

    #[test]
    fn test_calendar_entry_builder() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let entry = CalendarEntry::all_day(calendar_id, "Birthday", date)
            .with_description("John's birthday party")
            .with_location("123 Main St")
            .with_color("#F97316");

        assert_eq!(entry.calendar_id, calendar_id);
        assert_eq!(entry.title, "Birthday");
        assert_eq!(entry.description, Some("John's birthday party".to_string()));
        assert_eq!(entry.location, Some("123 Main St".to_string()));
        assert_eq!(entry.color, Some("#F97316".to_string()));
        assert_eq!(entry.date, date);
        assert!(entry.kind.is_all_day());
    }

    #[test]
    fn test_day_data() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let empty_day = DayData::empty(date);

        assert!(empty_day.is_empty());
        assert_eq!(empty_day.entry_count(), 0);

        let entry = CalendarEntry::all_day(calendar_id, "Test", date);
        let day_with_entry = DayData::new(date, vec![entry]);

        assert!(!day_with_entry.is_empty());
        assert_eq!(day_with_entry.entry_count(), 1);
    }
}
