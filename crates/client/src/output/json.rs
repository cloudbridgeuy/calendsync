//! JSON output formatting.

/// Format a value as JSON.
pub fn format_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}
