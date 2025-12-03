//! Client error types.

use thiserror::Error;

/// Result type alias for client module.
pub type Result<T> = std::result::Result<T, ClientError>;

/// Errors that can occur during client operations.
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Server returned {status}: {message}")]
    ServerError { status: u16, message: String },

    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("SSE parse error: {0}")]
    SseParse(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
