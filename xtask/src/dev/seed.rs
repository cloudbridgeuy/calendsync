//! HTTP-based seeding module for the development server.
//!
//! This module provides functionality to seed the server with demo data via HTTP.
//! It uses the `calendsync_core` library for data generation, ensuring consistency
//! with the domain types.
//!
//! ## Architecture
//!
//! Follows the Functional Core - Imperative Shell pattern:
//! - **Pure functions**: Data generation and conversion (`generate_seed_calendar`,
//!   `convert_entry_to_seed`, etc.)
//! - **I/O functions**: HTTP operations (`wait_for_server`, `create_calendar_via_http`,
//!   `seed_via_http`)
//!
//! ## Side Effect: API Validation
//!
//! As a side effect, seeding validates that the API endpoints work correctly.
//! If seeding succeeds, the API is functional.

use std::time::Duration;

use chrono::{NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use calendsync_core::calendar::{generate_seed_entries, CalendarEntry, EntryKind};

use super::error::{DevError, Result};

// ============================================================================
// Types
// ============================================================================

/// Calendar data for seeding via HTTP.
#[derive(Debug, Serialize)]
pub struct SeedCalendar {
    pub name: String,
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Entry data for seeding via HTTP.
/// Field names match the server's CreateEntry form expectations.
#[derive(Debug, Serialize)]
pub struct SeedEntry {
    pub calendar_id: Uuid,
    pub title: String,
    pub date: NaiveDate,
    pub entry_type: String, // "all_day", "timed", "task", "multi_day"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<NaiveTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<NaiveTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NaiveDate>,
}

/// Response from calendar creation endpoint.
#[derive(Debug, Deserialize)]
struct CalendarResponse {
    id: Uuid,
}

// ============================================================================
// Pure Functions (Functional Core)
// ============================================================================

/// Returns a default calendar for seeding.
pub fn generate_seed_calendar() -> SeedCalendar {
    SeedCalendar {
        name: "Demo Calendar".to_string(),
        color: "#3B82F6".to_string(), // Blue
        description: Some("A demo calendar with sample entries".to_string()),
    }
}

/// Converts an `EntryKind` to the form string expected by the server.
pub fn entry_type_string(kind: &EntryKind) -> &'static str {
    match kind {
        EntryKind::AllDay => "all_day",
        EntryKind::Timed { .. } => "timed",
        EntryKind::Task { .. } => "task",
        EntryKind::MultiDay { .. } => "multi_day",
    }
}

/// Converts a `CalendarEntry` to a `SeedEntry` for HTTP submission.
pub fn convert_entry_to_seed(entry: &CalendarEntry) -> SeedEntry {
    let entry_type = entry_type_string(&entry.kind).to_string();

    // Extract time fields from Timed variant
    let (start_time, end_time) = match &entry.kind {
        EntryKind::Timed { start, end } => (Some(*start), Some(*end)),
        _ => (None, None),
    };

    // Extract end_date from MultiDay variant
    let end_date = match &entry.kind {
        EntryKind::MultiDay { end, .. } => Some(*end),
        _ => None,
    };

    SeedEntry {
        calendar_id: entry.calendar_id,
        title: entry.title.clone(),
        date: entry.date,
        entry_type,
        description: entry.description.clone(),
        location: entry.location.clone(),
        color: entry.color.clone(),
        start_time,
        end_time,
        end_date,
    }
}

/// Generates seed entries for HTTP submission.
///
/// Uses `calendsync_core::calendar::generate_seed_entries` for data generation,
/// then converts each entry to the `SeedEntry` format.
pub fn generate_entries_for_seeding(
    calendar_id: Uuid,
    center_date: NaiveDate,
    count: u32,
) -> Vec<SeedEntry> {
    let entries = generate_seed_entries(calendar_id, center_date, count);
    entries.iter().map(convert_entry_to_seed).collect()
}

// ============================================================================
// I/O Functions (Imperative Shell)
// ============================================================================

/// Polls the server's health endpoint until it responds or times out.
///
/// # Arguments
///
/// * `base_url` - The server's base URL (e.g., "http://localhost:3000")
/// * `timeout` - Maximum time to wait for the server to become healthy
///
/// # Errors
///
/// Returns `DevError::ServerNotHealthy` if the server doesn't respond within the timeout.
pub async fn wait_for_server(base_url: &str, timeout: Duration) -> Result<()> {
    let client = reqwest::Client::new();
    let health_url = format!("{base_url}/healthz");
    let poll_interval = Duration::from_millis(500);
    let start = std::time::Instant::now();

    loop {
        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => {
                return Ok(());
            }
            _ => {
                if start.elapsed() >= timeout {
                    return Err(DevError::ServerNotHealthy {
                        timeout_secs: timeout.as_secs(),
                    });
                }
                tokio::time::sleep(poll_interval).await;
            }
        }
    }
}

/// Creates a calendar via the HTTP API.
///
/// # Arguments
///
/// * `client` - The HTTP client to use
/// * `base_url` - The server's base URL
/// * `calendar` - The calendar data to create
///
/// # Returns
///
/// The UUID of the created calendar.
///
/// # Errors
///
/// Returns `DevError::SeedingFailed` if the request fails.
pub async fn create_calendar_via_http(
    client: &reqwest::Client,
    base_url: &str,
    calendar: &SeedCalendar,
) -> Result<Uuid> {
    let url = format!("{base_url}/api/calendars");

    let response = client
        .post(&url)
        .form(calendar)
        .send()
        .await
        .map_err(|e| DevError::SeedingFailed(format!("Failed to create calendar: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(DevError::SeedingFailed(format!(
            "Failed to create calendar: {status} - {body}"
        )));
    }

    let calendar_response: CalendarResponse = response
        .json()
        .await
        .map_err(|e| DevError::SeedingFailed(format!("Failed to parse calendar response: {e}")))?;

    Ok(calendar_response.id)
}

/// Creates an entry via the HTTP API.
///
/// # Arguments
///
/// * `client` - The HTTP client to use
/// * `base_url` - The server's base URL
/// * `entry` - The entry data to create
///
/// # Errors
///
/// Returns `DevError::SeedingFailed` if the request fails.
pub async fn create_entry_via_http(
    client: &reqwest::Client,
    base_url: &str,
    entry: &SeedEntry,
) -> Result<()> {
    let url = format!("{base_url}/api/entries");

    let response = client
        .post(&url)
        .form(entry)
        .send()
        .await
        .map_err(|e| DevError::SeedingFailed(format!("Failed to create entry: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(DevError::SeedingFailed(format!(
            "Failed to create entry '{}': {status} - {body}",
            entry.title
        )));
    }

    Ok(())
}

/// Seeds the server with demo data via HTTP.
///
/// This is the main entry point for seeding. It:
/// 1. Creates a reqwest client
/// 2. Creates a demo calendar
/// 3. Generates and creates 25 mock entries
///
/// # Arguments
///
/// * `base_url` - The server's base URL (e.g., "http://localhost:3000")
/// * `silent` - If false, prints progress messages
///
/// # Returns
///
/// The UUID of the created calendar.
///
/// # Errors
///
/// Returns an error if any HTTP request fails.
pub async fn seed_via_http(base_url: &str, silent: bool) -> Result<Uuid> {
    let client = reqwest::Client::new();

    // Create calendar
    let calendar = generate_seed_calendar();
    let calendar_id = create_calendar_via_http(&client, base_url, &calendar).await?;

    if !silent {
        println!("Created calendar: {} ({})", calendar.name, calendar_id);
    }

    // Generate and create entries
    let center_date = Utc::now().date_naive();
    let entries = generate_entries_for_seeding(calendar_id, center_date, 25);
    let entry_count = entries.len();

    for entry in &entries {
        create_entry_via_http(&client, base_url, entry).await?;
    }

    if !silent {
        println!("Created {entry_count} entries");
    }

    Ok(calendar_id)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_generate_seed_calendar() {
        let calendar = generate_seed_calendar();

        assert_eq!(calendar.name, "Demo Calendar");
        assert_eq!(calendar.color, "#3B82F6");
        assert!(calendar.description.is_some());
    }

    #[test]
    fn test_entry_type_string() {
        assert_eq!(entry_type_string(&EntryKind::AllDay), "all_day");
        assert_eq!(
            entry_type_string(&EntryKind::Timed {
                start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            }),
            "timed"
        );
        assert_eq!(
            entry_type_string(&EntryKind::Task { completed: false }),
            "task"
        );
        assert_eq!(
            entry_type_string(&EntryKind::MultiDay {
                start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                end: NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
            }),
            "multi_day"
        );
    }

    #[test]
    fn test_convert_entry_to_seed_all_day() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = CalendarEntry::all_day(calendar_id, "Birthday", date)
            .with_description("John's birthday")
            .with_color("#EC4899");

        let seed = convert_entry_to_seed(&entry);

        assert_eq!(seed.calendar_id, calendar_id);
        assert_eq!(seed.title, "Birthday");
        assert_eq!(seed.date, date);
        assert_eq!(seed.entry_type, "all_day");
        assert_eq!(seed.description, Some("John's birthday".to_string()));
        assert_eq!(seed.color, Some("#EC4899".to_string()));
        assert!(seed.start_time.is_none());
        assert!(seed.end_time.is_none());
        assert!(seed.end_date.is_none());
    }

    #[test]
    fn test_convert_entry_to_seed_timed() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let start = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        let end = NaiveTime::from_hms_opt(11, 30, 0).unwrap();
        let entry = CalendarEntry::timed(calendar_id, "Meeting", date, start, end);

        let seed = convert_entry_to_seed(&entry);

        assert_eq!(seed.entry_type, "timed");
        assert_eq!(seed.start_time, Some(start));
        assert_eq!(seed.end_time, Some(end));
        assert!(seed.end_date.is_none());
    }

    #[test]
    fn test_convert_entry_to_seed_multi_day() {
        let calendar_id = Uuid::new_v4();
        let start = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 6, 18).unwrap();
        let entry = CalendarEntry::multi_day(calendar_id, "Vacation", start, end, start);

        let seed = convert_entry_to_seed(&entry);

        assert_eq!(seed.entry_type, "multi_day");
        assert_eq!(seed.date, start);
        assert_eq!(seed.end_date, Some(end));
        assert!(seed.start_time.is_none());
        assert!(seed.end_time.is_none());
    }

    #[test]
    fn test_convert_entry_to_seed_task() {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let entry = CalendarEntry::task(calendar_id, "Review PR", date, false);

        let seed = convert_entry_to_seed(&entry);

        assert_eq!(seed.entry_type, "task");
        assert!(seed.start_time.is_none());
        assert!(seed.end_time.is_none());
        assert!(seed.end_date.is_none());
    }

    #[test]
    fn test_generate_entries_for_seeding() {
        let calendar_id = Uuid::new_v4();
        let center = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        let entries = generate_entries_for_seeding(calendar_id, center, 10);

        assert_eq!(entries.len(), 10);
        for entry in &entries {
            assert_eq!(entry.calendar_id, calendar_id);
            assert!(!entry.title.is_empty());
            // Verify entry_type is valid
            assert!(
                entry.entry_type == "all_day"
                    || entry.entry_type == "timed"
                    || entry.entry_type == "task"
                    || entry.entry_type == "multi_day"
            );
        }
    }
}
