use thiserror::Error;

/// Errors that can occur when validating or manipulating calendars.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CalendarError {
    #[error("Calendar name cannot be empty")]
    EmptyName,
    #[error("Calendar name too long (max 100 characters)")]
    NameTooLong,
    #[error("Invalid color format: {0}")]
    InvalidColor(String),
}

/// Errors that can occur when validating or manipulating calendar entries.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EntryError {
    #[error("Entry title cannot be empty")]
    EmptyTitle,
    #[error("Entry title too long (max 200 characters)")]
    TitleTooLong,
    #[error("End date must be after or equal to start date")]
    InvalidDateRange,
    #[error("End time must be after start time")]
    InvalidTimeRange,
    #[error("Calendar ID is required")]
    MissingCalendarId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_error_display() {
        assert_eq!(
            CalendarError::EmptyName.to_string(),
            "Calendar name cannot be empty"
        );
        assert_eq!(
            CalendarError::InvalidColor("#xyz".to_string()).to_string(),
            "Invalid color format: #xyz"
        );
    }

    #[test]
    fn test_entry_error_display() {
        assert_eq!(
            EntryError::EmptyTitle.to_string(),
            "Entry title cannot be empty"
        );
        assert_eq!(
            EntryError::InvalidDateRange.to_string(),
            "End date must be after or equal to start date"
        );
    }
}
