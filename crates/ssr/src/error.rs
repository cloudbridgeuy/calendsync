//! SSR errors including I/O operations.

use calendsync_ssr_core::SsrCoreError;
use thiserror::Error;

/// SSR errors including I/O operations.
#[derive(Error, Debug)]
pub enum SsrError {
    #[error("Core error: {0}")]
    Core(#[from] SsrCoreError),

    #[error("Failed to load server bundle from {path}: {reason}")]
    BundleLoad { path: String, reason: String },

    #[error("JavaScript execution error: {0}")]
    JsExecution(String),

    #[error("No HTML was rendered by React")]
    NoHtmlRendered,

    #[error("Worker channel closed")]
    ChannelClosed,

    #[error("Render timeout after {0}ms")]
    Timeout(u64),

    #[error("Service overloaded, retry after {retry_after_secs}s")]
    Overloaded { retry_after_secs: u32 },
}

pub type Result<T> = std::result::Result<T, SsrError>;

/// Sanitize error messages for client-facing responses.
///
/// Hides internal details while providing useful feedback.
pub fn sanitize_error(error: &SsrError) -> String {
    match error {
        // Safe to expose
        SsrError::Timeout(ms) => format!("Render timed out after {ms}ms"),
        SsrError::ChannelClosed => "Service temporarily unavailable".to_string(),
        SsrError::Overloaded { retry_after_secs } => {
            format!("Service busy, retry after {retry_after_secs}s")
        }
        // Hide internal details
        SsrError::BundleLoad { .. } => "Internal configuration error".to_string(),
        SsrError::JsExecution(_) => "Render failed".to_string(),
        SsrError::NoHtmlRendered => "Render produced no output".to_string(),
        SsrError::Core(_) => "Invalid request".to_string(),
    }
}
