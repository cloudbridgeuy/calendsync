use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user who can access calendars.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Creates a new user with a generated UUID and current timestamps.
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            email: email.into(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets a specific ID for this user (useful for testing).
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    /// Sets a specific created_at timestamp (useful for testing).
    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }

    /// Sets a specific updated_at timestamp (useful for testing).
    pub fn with_updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = updated_at;
        self
    }

    /// Updates the updated_at timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

/// Role for calendar membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CalendarRole {
    /// Can read/write entries and administer the calendar (delete, invite users).
    Owner,
    /// Can read and write calendar entries.
    Writer,
    /// Can only read calendar entries.
    Reader,
}

impl CalendarRole {
    /// Returns true if this role can write entries.
    pub fn can_write(&self) -> bool {
        matches!(self, CalendarRole::Owner | CalendarRole::Writer)
    }

    /// Returns true if this role can administer the calendar.
    pub fn can_administer(&self) -> bool {
        matches!(self, CalendarRole::Owner)
    }
}

/// Membership linking a user to a calendar with a role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarMembership {
    pub calendar_id: Uuid,
    pub user_id: Uuid,
    pub role: CalendarRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CalendarMembership {
    /// Creates a new membership with the current timestamp.
    pub fn new(calendar_id: Uuid, user_id: Uuid, role: CalendarRole) -> Self {
        let now = Utc::now();
        Self {
            calendar_id,
            user_id,
            role,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new owner membership.
    pub fn owner(calendar_id: Uuid, user_id: Uuid) -> Self {
        Self::new(calendar_id, user_id, CalendarRole::Owner)
    }

    /// Creates a new writer membership.
    pub fn writer(calendar_id: Uuid, user_id: Uuid) -> Self {
        Self::new(calendar_id, user_id, CalendarRole::Writer)
    }

    /// Creates a new reader membership.
    pub fn reader(calendar_id: Uuid, user_id: Uuid) -> Self {
        Self::new(calendar_id, user_id, CalendarRole::Reader)
    }

    /// Sets a specific created_at timestamp (useful for testing).
    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }

    /// Sets a specific updated_at timestamp (useful for testing).
    pub fn with_updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = updated_at;
        self
    }

    /// Updates the updated_at timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

/// A named calendar that contains entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calendar {
    pub id: Uuid,
    pub name: String,
    /// Default color for entries in this calendar (CSS color value).
    pub color: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Calendar {
    /// Creates a new calendar with the given name and color.
    pub fn new(name: impl Into<String>, color: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            color: color.into(),
            description: None,
            created_at: now,
            updated_at: now,
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

    /// Sets a specific created_at timestamp (useful for testing).
    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }

    /// Sets a specific updated_at timestamp (useful for testing).
    pub fn with_updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = updated_at;
        self
    }

    /// Updates the updated_at timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            calendar_id,
            title: title.into(),
            description: None,
            location: None,
            kind: EntryKind::MultiDay { start, end },
            date: display_date,
            color: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new all-day event.
    pub fn all_day(calendar_id: Uuid, title: impl Into<String>, date: NaiveDate) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            calendar_id,
            title: title.into(),
            description: None,
            location: None,
            kind: EntryKind::AllDay,
            date,
            color: None,
            created_at: now,
            updated_at: now,
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
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            calendar_id,
            title: title.into(),
            description: None,
            location: None,
            kind: EntryKind::Timed { start, end },
            date,
            color: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new task.
    pub fn task(
        calendar_id: Uuid,
        title: impl Into<String>,
        date: NaiveDate,
        completed: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            calendar_id,
            title: title.into(),
            description: None,
            location: None,
            kind: EntryKind::Task { completed },
            date,
            color: None,
            created_at: now,
            updated_at: now,
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

    /// Sets a specific created_at timestamp (useful for testing).
    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }

    /// Sets a specific updated_at timestamp (useful for testing).
    pub fn with_updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = updated_at;
        self
    }

    /// Updates the updated_at timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
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

/// SSE event types for real-time calendar updates.
///
/// These events are sent from the server to clients via Server-Sent Events (SSE)
/// when calendar entries are created, updated, or deleted.
///
/// The `date` field is included to help clients update their view without
/// needing to re-query for entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CalendarEvent {
    /// A new entry was added to the calendar.
    EntryAdded {
        /// The newly created entry.
        entry: CalendarEntry,
        /// The date string for client-side view updates.
        date: String,
    },
    /// An existing entry was updated.
    EntryUpdated {
        /// The updated entry.
        entry: CalendarEntry,
        /// The date string for client-side view updates.
        date: String,
    },
    /// An entry was deleted from the calendar.
    EntryDeleted {
        /// The ID of the deleted entry.
        entry_id: Uuid,
        /// The date string for client-side view updates.
        date: String,
    },
}

impl CalendarEvent {
    /// Creates an EntryAdded event.
    pub fn entry_added(entry: CalendarEntry) -> Self {
        let date = entry.date.to_string();
        Self::EntryAdded { entry, date }
    }

    /// Creates an EntryUpdated event.
    pub fn entry_updated(entry: CalendarEntry) -> Self {
        let date = entry.date.to_string();
        Self::EntryUpdated { entry, date }
    }

    /// Creates an EntryDeleted event.
    pub fn entry_deleted(entry_id: Uuid, date: NaiveDate) -> Self {
        Self::EntryDeleted {
            entry_id,
            date: date.to_string(),
        }
    }

    /// Returns the calendar entry if this is an add or update event.
    pub fn entry(&self) -> Option<&CalendarEntry> {
        match self {
            Self::EntryAdded { entry, .. } | Self::EntryUpdated { entry, .. } => Some(entry),
            Self::EntryDeleted { .. } => None,
        }
    }

    /// Returns the date string associated with this event.
    pub fn date(&self) -> &str {
        match self {
            Self::EntryAdded { date, .. }
            | Self::EntryUpdated { date, .. }
            | Self::EntryDeleted { date, .. } => date,
        }
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
