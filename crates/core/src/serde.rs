//! Serde helper functions for form deserialization.
//!
//! These functions handle the quirks of HTML form submissions where
//! empty strings should be treated as None for optional fields.

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Deserializer};

/// Deserialize an optional string, treating empty strings as None.
pub fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.filter(|s| !s.trim().is_empty()))
}

/// Deserialize an optional NaiveDate, treating empty strings as None.
/// Expects format: YYYY-MM-DD
pub fn deserialize_optional_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
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
/// Accepts formats: HH:MM or HH:MM:SS
pub fn deserialize_optional_time<'de, D>(deserializer: D) -> Result<Option<NaiveTime>, D::Error>
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test struct that uses the deserializer functions
    #[derive(Debug, Deserialize, PartialEq)]
    struct TestStruct {
        #[serde(default, deserialize_with = "deserialize_optional_string")]
        string_field: Option<String>,
        #[serde(default, deserialize_with = "deserialize_optional_date")]
        date_field: Option<NaiveDate>,
        #[serde(default, deserialize_with = "deserialize_optional_time")]
        time_field: Option<NaiveTime>,
    }

    #[test]
    fn test_deserialize_optional_string_empty() {
        let json = r#"{"string_field": ""}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(result.string_field, None);
    }

    #[test]
    fn test_deserialize_optional_string_whitespace() {
        let json = r#"{"string_field": "   "}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(result.string_field, None);
    }

    #[test]
    fn test_deserialize_optional_string_value() {
        let json = r#"{"string_field": "hello"}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(result.string_field, Some("hello".to_string()));
    }

    #[test]
    fn test_deserialize_optional_string_missing() {
        let json = r#"{}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(result.string_field, None);
    }

    #[test]
    fn test_deserialize_optional_date_valid() {
        let json = r#"{"date_field": "2025-01-15"}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(
            result.date_field,
            Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap())
        );
    }

    #[test]
    fn test_deserialize_optional_date_empty() {
        let json = r#"{"date_field": ""}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(result.date_field, None);
    }

    #[test]
    fn test_deserialize_optional_date_invalid() {
        let json = r#"{"date_field": "not-a-date"}"#;
        let result: Result<TestStruct, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_optional_time_hhmm() {
        let json = r#"{"time_field": "14:30"}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(
            result.time_field,
            Some(NaiveTime::from_hms_opt(14, 30, 0).unwrap())
        );
    }

    #[test]
    fn test_deserialize_optional_time_hhmmss() {
        let json = r#"{"time_field": "14:30:45"}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(
            result.time_field,
            Some(NaiveTime::from_hms_opt(14, 30, 45).unwrap())
        );
    }

    #[test]
    fn test_deserialize_optional_time_empty() {
        let json = r#"{"time_field": ""}"#;
        let result: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(result.time_field, None);
    }

    #[test]
    fn test_deserialize_optional_time_invalid() {
        let json = r#"{"time_field": "not-a-time"}"#;
        let result: Result<TestStruct, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
