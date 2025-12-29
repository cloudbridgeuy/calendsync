//! API request types for calendar operations.
//!
//! These types are shared between the server and client for type-safe API communication.
//! Following the Functional Core pattern, these are pure data types with no I/O.

use chrono::{NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::{Calendar, CalendarEntry, EntryKind};

/// Entry type discriminant for API requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryType {
    AllDay,
    Timed,
    Task,
    MultiDay,
}

impl EntryType {
    /// Convert an EntryKind to its EntryType discriminant.
    pub fn from_kind(kind: &EntryKind) -> Self {
        match kind {
            EntryKind::AllDay => EntryType::AllDay,
            EntryKind::Timed { .. } => EntryType::Timed,
            EntryKind::Task { .. } => EntryType::Task,
            EntryKind::MultiDay => EntryType::MultiDay,
        }
    }
}

/// Request payload for creating a new calendar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCalendarRequest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl CreateCalendarRequest {
    /// Create a new request with just a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            color: None,
            description: None,
        }
    }

    /// Set the calendar color.
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the calendar description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Convert into a Calendar, using a default color if none specified.
    pub fn into_calendar(self) -> Calendar {
        let color = self.color.unwrap_or_else(|| "#3B82F6".to_string());
        let mut calendar = Calendar::new(self.name, color);
        if let Some(description) = self.description {
            calendar = calendar.with_description(description);
        }
        calendar
    }
}

/// Request payload for updating a calendar.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateCalendarRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl UpdateCalendarRequest {
    /// Create an empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the calendar name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the calendar color.
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the calendar description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Apply updates to an existing calendar.
    pub fn apply_to(self, calendar: &mut Calendar) {
        if let Some(name) = self.name {
            calendar.name = name;
        }
        if let Some(color) = self.color {
            calendar.color = color;
        }
        if let Some(description) = self.description {
            calendar.description = Some(description);
        }
    }
}

/// Request payload for creating a new entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntryRequest {
    pub calendar_id: Uuid,
    pub title: String,
    pub start_date: NaiveDate,
    pub entry_type: EntryType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<NaiveTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<NaiveTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl CreateEntryRequest {
    /// Create an all-day entry request.
    pub fn all_day(calendar_id: Uuid, title: impl Into<String>, start_date: NaiveDate) -> Self {
        Self {
            calendar_id,
            title: title.into(),
            start_date,
            entry_type: EntryType::AllDay,
            description: None,
            location: None,
            start_time: None,
            end_time: None,
            end_date: None,
            color: None,
        }
    }

    /// Create a timed entry request.
    pub fn timed(
        calendar_id: Uuid,
        title: impl Into<String>,
        start_date: NaiveDate,
        start: NaiveTime,
        end: NaiveTime,
    ) -> Self {
        Self {
            calendar_id,
            title: title.into(),
            start_date,
            entry_type: EntryType::Timed,
            description: None,
            location: None,
            start_time: Some(start),
            end_time: Some(end),
            end_date: None,
            color: None,
        }
    }

    /// Create a task entry request.
    pub fn task(calendar_id: Uuid, title: impl Into<String>, start_date: NaiveDate) -> Self {
        Self {
            calendar_id,
            title: title.into(),
            start_date,
            entry_type: EntryType::Task,
            description: None,
            location: None,
            start_time: None,
            end_time: None,
            end_date: None,
            color: None,
        }
    }

    /// Create a multi-day entry request.
    pub fn multi_day(
        calendar_id: Uuid,
        title: impl Into<String>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Self {
        Self {
            calendar_id,
            title: title.into(),
            start_date,
            entry_type: EntryType::MultiDay,
            description: None,
            location: None,
            start_time: None,
            end_time: None,
            end_date: Some(end_date),
            color: None,
        }
    }

    /// Set the entry description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the entry location.
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Set the entry color.
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Convert into a CalendarEntry.
    /// Returns None if required fields for the entry type are missing.
    pub fn into_entry(self) -> Option<CalendarEntry> {
        let end_date = match self.entry_type {
            EntryType::MultiDay => self.end_date?,
            _ => self.start_date,
        };

        let kind = match self.entry_type {
            EntryType::AllDay => EntryKind::AllDay,
            EntryType::Timed => {
                let start = self.start_time?;
                let end = self.end_time?;
                EntryKind::Timed { start, end }
            }
            EntryType::Task => EntryKind::Task { completed: false },
            EntryType::MultiDay => EntryKind::MultiDay,
        };

        let now = Utc::now();
        Some(CalendarEntry {
            id: Uuid::new_v4(),
            calendar_id: self.calendar_id,
            title: self.title,
            description: self.description,
            location: self.location,
            kind,
            start_date: self.start_date,
            end_date,
            color: self.color,
            created_at: now,
            updated_at: now,
        })
    }
}

/// Request payload for updating an entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateEntryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_type: Option<EntryType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<NaiveTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<NaiveTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
}

impl UpdateEntryRequest {
    /// Create an empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the entry title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the entry start date.
    pub fn with_start_date(mut self, start_date: NaiveDate) -> Self {
        self.start_date = Some(start_date);
        self
    }

    /// Set the entry type.
    pub fn with_entry_type(mut self, entry_type: EntryType) -> Self {
        self.entry_type = Some(entry_type);
        self
    }

    /// Set the entry description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the entry location.
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Set the entry start time (for timed entries).
    pub fn with_start_time(mut self, start: NaiveTime) -> Self {
        self.start_time = Some(start);
        self
    }

    /// Set the entry end time (for timed entries).
    pub fn with_end_time(mut self, end: NaiveTime) -> Self {
        self.end_time = Some(end);
        self
    }

    /// Set the entry end date (for multi-day entries).
    pub fn with_end_date(mut self, end: NaiveDate) -> Self {
        self.end_date = Some(end);
        self
    }

    /// Set the entry color.
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the task completion status.
    pub fn with_completed(mut self, completed: bool) -> Self {
        self.completed = Some(completed);
        self
    }

    /// Apply updates to an existing entry.
    pub fn apply_to(self, entry: &mut CalendarEntry) {
        if let Some(title) = self.title {
            entry.title = title;
        }
        if let Some(description) = self.description {
            entry.description = Some(description);
        }
        if let Some(location) = self.location {
            entry.location = Some(location);
        }
        if let Some(start_date) = self.start_date {
            entry.start_date = start_date;
            // For non-multi-day entries, keep start_date == end_date
            if !entry.kind.is_multi_day() {
                entry.end_date = start_date;
            }
        }
        if let Some(end_date) = self.end_date {
            entry.end_date = end_date;
        }
        if let Some(color) = self.color {
            entry.color = Some(color);
        }

        // Handle entry type changes
        if let Some(entry_type) = self.entry_type {
            entry.kind = match entry_type {
                EntryType::AllDay => {
                    // For AllDay, ensure start_date == end_date
                    entry.end_date = entry.start_date;
                    EntryKind::AllDay
                }
                EntryType::Timed => {
                    // For Timed, ensure start_date == end_date
                    entry.end_date = entry.start_date;
                    let start = self.start_time.unwrap_or_else(|| {
                        entry
                            .kind
                            .start_time()
                            .unwrap_or_else(|| NaiveTime::from_hms_opt(9, 0, 0).unwrap())
                    });
                    let end = self.end_time.unwrap_or_else(|| {
                        entry
                            .kind
                            .end_time()
                            .unwrap_or_else(|| NaiveTime::from_hms_opt(10, 0, 0).unwrap())
                    });
                    EntryKind::Timed { start, end }
                }
                EntryType::Task => {
                    // For Task, ensure start_date == end_date
                    entry.end_date = entry.start_date;
                    let completed = self.completed.unwrap_or(false);
                    EntryKind::Task { completed }
                }
                EntryType::MultiDay => {
                    // For MultiDay, set end_date from request or default to existing end_date
                    if let Some(new_end_date) = self.end_date {
                        entry.end_date = new_end_date;
                    }
                    // If end_date <= start_date, default to start_date + 1 day
                    if entry.end_date <= entry.start_date {
                        entry.end_date = entry.start_date;
                    }
                    EntryKind::MultiDay
                }
            };
        } else {
            // Update time fields within existing entry type
            match &mut entry.kind {
                EntryKind::Timed { start, end } => {
                    if let Some(new_start) = self.start_time {
                        *start = new_start;
                    }
                    if let Some(new_end) = self.end_time {
                        *end = new_end;
                    }
                }
                EntryKind::Task { completed } => {
                    if let Some(new_completed) = self.completed {
                        *completed = new_completed;
                    }
                }
                _ => {}
            }
        }
    }
}

/// Query parameters for listing entries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListEntriesQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlighted_day: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<u32>,
}

impl ListEntriesQuery {
    /// Create an empty query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by calendar ID.
    pub fn for_calendar(mut self, calendar_id: Uuid) -> Self {
        self.calendar_id = Some(calendar_id);
        self
    }

    /// Filter by date range.
    pub fn with_range(mut self, start: NaiveDate, end: NaiveDate) -> Self {
        self.start = Some(start);
        self.end = Some(end);
        self
    }

    /// Set highlighted day and relative range.
    pub fn with_highlighted_day(mut self, day: NaiveDate, before: u32, after: u32) -> Self {
        self.highlighted_day = Some(day);
        self.before = Some(before);
        self.after = Some(after);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_calendar_request() {
        let req = CreateCalendarRequest::new("Work")
            .with_color("#FF0000")
            .with_description("Work calendar");

        assert_eq!(req.name, "Work");
        assert_eq!(req.color, Some("#FF0000".to_string()));
        assert_eq!(req.description, Some("Work calendar".to_string()));
    }

    #[test]
    fn test_create_calendar_into_calendar() {
        let req = CreateCalendarRequest::new("Personal");
        let calendar = req.into_calendar();

        assert_eq!(calendar.name, "Personal");
        assert_eq!(calendar.color, "#3B82F6"); // default blue
    }

    #[test]
    fn test_update_calendar_apply() {
        let mut calendar = Calendar::new("Old Name", "#000000");
        let update = UpdateCalendarRequest::new()
            .with_name("New Name")
            .with_color("#FFFFFF");

        update.apply_to(&mut calendar);

        assert_eq!(calendar.name, "New Name");
        assert_eq!(calendar.color, "#FFFFFF");
    }

    #[test]
    fn test_create_entry_all_day() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let req = CreateEntryRequest::all_day(calendar_id, "Birthday", date)
            .with_description("Party time");

        let entry = req.into_entry().unwrap();
        assert_eq!(entry.title, "Birthday");
        assert!(matches!(entry.kind, EntryKind::AllDay));
    }

    #[test]
    fn test_create_entry_timed() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let start = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let end = NaiveTime::from_hms_opt(10, 0, 0).unwrap();

        let req = CreateEntryRequest::timed(calendar_id, "Meeting", date, start, end);
        let entry = req.into_entry().unwrap();

        assert_eq!(entry.title, "Meeting");
        assert!(matches!(entry.kind, EntryKind::Timed { .. }));
    }

    #[test]
    fn test_create_entry_timed_missing_times() {
        let calendar_id = Uuid::new_v4();
        let start_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        // This creates a timed request but without times
        let req = CreateEntryRequest {
            calendar_id,
            title: "Meeting".to_string(),
            start_date,
            entry_type: EntryType::Timed,
            description: None,
            location: None,
            start_time: None, // Missing!
            end_time: None,   // Missing!
            end_date: None,
            color: None,
        };

        assert!(req.into_entry().is_none()); // Should fail
    }

    #[test]
    fn test_update_entry_apply() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let mut entry = CalendarEntry::all_day(calendar_id, "Original", date);

        let update = UpdateEntryRequest::new()
            .with_title("Updated")
            .with_description("New description");

        update.apply_to(&mut entry);

        assert_eq!(entry.title, "Updated");
        assert_eq!(entry.description, Some("New description".to_string()));
    }

    #[test]
    fn test_list_entries_query() {
        let calendar_id = Uuid::new_v4();
        let start = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();

        let query = ListEntriesQuery::new()
            .for_calendar(calendar_id)
            .with_range(start, end);

        assert_eq!(query.calendar_id, Some(calendar_id));
        assert_eq!(query.start, Some(start));
        assert_eq!(query.end, Some(end));
    }

    #[test]
    fn test_entry_type_from_kind() {
        assert_eq!(EntryType::from_kind(&EntryKind::AllDay), EntryType::AllDay);
        assert_eq!(
            EntryType::from_kind(&EntryKind::Timed {
                start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            }),
            EntryType::Timed
        );
        assert_eq!(
            EntryType::from_kind(&EntryKind::Task { completed: false }),
            EntryType::Task
        );
        assert_eq!(
            EntryType::from_kind(&EntryKind::MultiDay),
            EntryType::MultiDay
        );
    }
}
