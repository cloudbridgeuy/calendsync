//! Last-Write-Wins (LWW) merge strategy for calendar entries.
//!
//! This module provides pure functions for merging concurrent updates to calendar entries
//! using the Last-Write-Wins conflict resolution strategy. The entry with the more recent
//! `updated_at` timestamp wins.
//!
//! This is part of the Functional Core - all functions are pure with no side effects.

use super::types::CalendarEntry;

/// Result of merging two calendar entries using LWW strategy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeResult {
    /// The client's entry should be used (client timestamp is strictly newer).
    ClientWins(CalendarEntry),
    /// The server's entry should be kept (server timestamp is same or newer).
    ServerWins(CalendarEntry),
}

impl MergeResult {
    /// Returns the winning entry.
    pub fn entry(&self) -> &CalendarEntry {
        match self {
            MergeResult::ClientWins(entry) | MergeResult::ServerWins(entry) => entry,
        }
    }

    /// Consumes the result and returns the winning entry.
    pub fn into_entry(self) -> CalendarEntry {
        match self {
            MergeResult::ClientWins(entry) | MergeResult::ServerWins(entry) => entry,
        }
    }

    /// Returns true if the client won the merge.
    pub fn is_client_win(&self) -> bool {
        matches!(self, MergeResult::ClientWins(_))
    }

    /// Returns true if the server won the merge.
    pub fn is_server_win(&self) -> bool {
        matches!(self, MergeResult::ServerWins(_))
    }
}

/// Merges a server entry with a client entry using Last-Write-Wins strategy.
///
/// The entry with the more recent `updated_at` timestamp wins. In case of a tie
/// (same timestamp), the server entry wins to ensure deterministic behavior.
///
/// # Arguments
///
/// * `server` - The current entry stored on the server
/// * `client` - The proposed entry from the client
///
/// # Returns
///
/// A `MergeResult` indicating which entry won and containing the winning entry.
///
/// # Examples
///
/// ```
/// use chrono::{NaiveDate, Utc, Duration};
/// use uuid::Uuid;
/// use calendsync_core::calendar::{CalendarEntry, merge_entry, MergeResult};
///
/// let calendar_id = Uuid::new_v4();
/// let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
/// let now = Utc::now();
///
/// let server = CalendarEntry::all_day(calendar_id, "Server", date)
///     .with_updated_at(now);
/// let client = CalendarEntry::all_day(calendar_id, "Client", date)
///     .with_updated_at(now + Duration::seconds(1));
///
/// match merge_entry(&server, &client) {
///     MergeResult::ClientWins(entry) => assert_eq!(entry.title, "Client"),
///     MergeResult::ServerWins(_) => panic!("Expected client to win"),
/// }
/// ```
pub fn merge_entry(server: &CalendarEntry, client: &CalendarEntry) -> MergeResult {
    if client.updated_at > server.updated_at {
        MergeResult::ClientWins(client.clone())
    } else {
        MergeResult::ServerWins(server.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, NaiveDate, Utc};
    use uuid::Uuid;

    fn make_entry(title: &str, updated_at: chrono::DateTime<Utc>) -> CalendarEntry {
        let calendar_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        CalendarEntry::all_day(calendar_id, title, date).with_updated_at(updated_at)
    }

    #[test]
    fn test_client_wins_when_newer() {
        let now = Utc::now();
        let server = make_entry("Server Entry", now);
        let client = make_entry("Client Entry", now + Duration::seconds(1));

        let result = merge_entry(&server, &client);

        assert!(result.is_client_win());
        assert!(!result.is_server_win());
        assert_eq!(result.entry().title, "Client Entry");
    }

    #[test]
    fn test_server_wins_when_same() {
        let now = Utc::now();
        let server = make_entry("Server Entry", now);
        let client = make_entry("Client Entry", now);

        let result = merge_entry(&server, &client);

        assert!(result.is_server_win());
        assert!(!result.is_client_win());
        assert_eq!(result.entry().title, "Server Entry");
    }

    #[test]
    fn test_server_wins_when_older() {
        let now = Utc::now();
        let server = make_entry("Server Entry", now);
        let client = make_entry("Client Entry", now - Duration::seconds(1));

        let result = merge_entry(&server, &client);

        assert!(result.is_server_win());
        assert_eq!(result.entry().title, "Server Entry");
    }

    #[test]
    fn test_into_entry_consumes_result() {
        let now = Utc::now();
        let server = make_entry("Server Entry", now);
        let client = make_entry("Client Entry", now + Duration::seconds(1));

        let result = merge_entry(&server, &client);
        let entry = result.into_entry();

        assert_eq!(entry.title, "Client Entry");
    }

    #[test]
    fn test_merge_result_clone() {
        let now = Utc::now();
        let server = make_entry("Server Entry", now);
        let client = make_entry("Client Entry", now + Duration::seconds(1));

        let result = merge_entry(&server, &client);
        let cloned = result.clone();

        assert_eq!(result, cloned);
    }
}
