//! Core SSR error types (pure - no I/O variants).

use thiserror::Error;

/// Maximum size for initial_data JSON (5MB).
pub const MAX_INITIAL_DATA_SIZE: usize = 5 * 1024 * 1024;

/// Core SSR errors (pure - no I/O variants).
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SsrCoreError {
    #[error("Invalid config: {0}")]
    InvalidConfig(String),

    #[error("Config serialization failed: {0}")]
    Serialization(String),

    #[error("Worker count must be at least 1")]
    InvalidWorkerCount,

    #[error("Render timeout must be positive")]
    InvalidTimeout,

    #[error("Payload too large: {size} bytes (max: {max} bytes)")]
    PayloadTooLarge { size: usize, max: usize },
}

pub type Result<T> = std::result::Result<T, SsrCoreError>;
