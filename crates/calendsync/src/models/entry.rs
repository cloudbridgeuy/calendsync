use chrono::{NaiveDate, NaiveTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use calendsync_core::calendar::{CalendarEntry, EntryKind, EntryType};
use calendsync_core::serde::{
    deserialize_optional_date, deserialize_optional_string, deserialize_optional_time,
};

/// Server-side entry type with custom deserialization.
///
/// This wraps the core `EntryType` to provide backwards compatibility
/// with existing form submissions.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerEntryType {
    AllDay,
    Timed,
    Task,
    MultiDay,
}

impl From<ServerEntryType> for EntryType {
    fn from(t: ServerEntryType) -> Self {
        match t {
            ServerEntryType::AllDay => EntryType::AllDay,
            ServerEntryType::Timed => EntryType::Timed,
            ServerEntryType::Task => EntryType::Task,
            ServerEntryType::MultiDay => EntryType::MultiDay,
        }
    }
}

/// Server-side request payload for creating a new entry.
///
/// This wraps the core `CreateEntryRequest` with server-specific custom
/// deserializers for form handling (empty strings → None).
#[derive(Debug, Deserialize)]
pub struct CreateEntry {
    pub calendar_id: Uuid,
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub location: Option<String>,
    pub start_date: NaiveDate,
    pub entry_type: ServerEntryType,
    #[serde(default, deserialize_with = "deserialize_optional_time")]
    pub start_time: Option<NaiveTime>,
    #[serde(default, deserialize_with = "deserialize_optional_time")]
    pub end_time: Option<NaiveTime>,
    #[serde(default, deserialize_with = "deserialize_optional_date")]
    pub end_date: Option<NaiveDate>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub color: Option<String>,
}

impl CreateEntry {
    /// Converts the create request into a CalendarEntry.
    /// Returns None if the entry type requires fields that are not provided.
    pub fn into_entry(self) -> Option<CalendarEntry> {
        let end_date = match self.entry_type {
            ServerEntryType::MultiDay => self.end_date?,
            _ => self.start_date,
        };

        let kind = match self.entry_type {
            ServerEntryType::AllDay => EntryKind::AllDay,
            ServerEntryType::Timed => {
                let start = self.start_time?;
                let end = self.end_time?;
                EntryKind::Timed { start, end }
            }
            ServerEntryType::Task => EntryKind::Task { completed: false },
            ServerEntryType::MultiDay => EntryKind::MultiDay,
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

/// Server-side request payload for updating an entry.
///
/// This wraps the core `UpdateEntryRequest` with server-specific custom
/// deserializers for form handling (empty strings → None).
#[derive(Debug, Deserialize)]
pub struct UpdateEntry {
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub location: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_date")]
    pub start_date: Option<NaiveDate>,
    #[serde(default)]
    pub entry_type: Option<ServerEntryType>,
    #[serde(default, deserialize_with = "deserialize_optional_time")]
    pub start_time: Option<NaiveTime>,
    #[serde(default, deserialize_with = "deserialize_optional_time")]
    pub end_time: Option<NaiveTime>,
    #[serde(default, deserialize_with = "deserialize_optional_date")]
    pub end_date: Option<NaiveDate>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub color: Option<String>,
    #[serde(default)]
    pub completed: Option<bool>,
}

impl UpdateEntry {
    /// Applies the update to an existing entry.
    pub fn apply_to(self, entry: &mut CalendarEntry) {
        // Update the updated_at timestamp
        entry.updated_at = Utc::now();

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
            // For non-multi-day entries, keep end_date in sync
            if !matches!(entry.kind, EntryKind::MultiDay) {
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
                ServerEntryType::AllDay => {
                    // When changing to all-day, sync end_date to start_date
                    entry.end_date = entry.start_date;
                    EntryKind::AllDay
                }
                ServerEntryType::Timed => {
                    // When changing to timed, sync end_date to start_date
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
                ServerEntryType::Task => {
                    // When changing to task, sync end_date to start_date
                    entry.end_date = entry.start_date;
                    let completed = self.completed.unwrap_or(false);
                    EntryKind::Task { completed }
                }
                ServerEntryType::MultiDay => {
                    // When changing to multi-day, use provided end_date or keep existing
                    if let Some(end) = self.end_date {
                        entry.end_date = end;
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
