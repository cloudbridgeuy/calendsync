use chrono::{NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Deserializer};
use uuid::Uuid;

use calendsync_core::calendar::{CalendarEntry, EntryKind, EntryType};

/// Deserialize an optional string, treating empty strings as None.
fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.filter(|s| !s.trim().is_empty()))
}

/// Deserialize an optional NaiveDate, treating empty strings as None.
fn deserialize_optional_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) if !s.trim().is_empty() => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map(Some)
            .map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

/// Deserialize an optional NaiveTime, treating empty strings as None.
fn deserialize_optional_time<'de, D>(deserializer: D) -> Result<Option<NaiveTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) if !s.trim().is_empty() => NaiveTime::parse_from_str(&s, "%H:%M")
            .or_else(|_| NaiveTime::parse_from_str(&s, "%H:%M:%S"))
            .map(Some)
            .map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

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
    pub date: NaiveDate,
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
        let kind = match self.entry_type {
            ServerEntryType::AllDay => EntryKind::AllDay,
            ServerEntryType::Timed => {
                let start = self.start_time?;
                let end = self.end_time?;
                EntryKind::Timed { start, end }
            }
            ServerEntryType::Task => EntryKind::Task { completed: false },
            ServerEntryType::MultiDay => {
                let end = self.end_date?;
                EntryKind::MultiDay {
                    start: self.date,
                    end,
                }
            }
        };

        let now = Utc::now();
        let mut entry = CalendarEntry {
            id: Uuid::new_v4(),
            calendar_id: self.calendar_id,
            title: self.title,
            description: self.description,
            location: self.location,
            kind,
            date: self.date,
            color: self.color,
            created_at: now,
            updated_at: now,
        };

        // If no custom color, entry will use calendar's default
        if entry.color.is_none() {
            entry.color = None;
        }

        Some(entry)
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
    pub date: Option<NaiveDate>,
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
        if let Some(date) = self.date {
            entry.date = date;
        }
        if let Some(color) = self.color {
            entry.color = Some(color);
        }

        // Handle entry type changes
        if let Some(entry_type) = self.entry_type {
            entry.kind = match entry_type {
                ServerEntryType::AllDay => EntryKind::AllDay,
                ServerEntryType::Timed => {
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
                    let completed = self.completed.unwrap_or(false);
                    EntryKind::Task { completed }
                }
                ServerEntryType::MultiDay => {
                    let start = self.date.unwrap_or(entry.date);
                    let end = self
                        .end_date
                        .unwrap_or_else(|| entry.kind.multi_day_end().unwrap_or(start));
                    EntryKind::MultiDay { start, end }
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
                EntryKind::MultiDay { end, .. } => {
                    if let Some(new_end) = self.end_date {
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
