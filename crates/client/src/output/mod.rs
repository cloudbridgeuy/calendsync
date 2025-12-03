//! Output formatting functions.

pub mod json;
pub mod pretty;

use crate::cli::OutputFormat;

/// Format a value for output.
pub fn format_output<T: serde::Serialize>(value: &T, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => json::format_json(value),
        OutputFormat::Pretty => serde_json::to_string_pretty(value).unwrap_or_default(),
    }
}
