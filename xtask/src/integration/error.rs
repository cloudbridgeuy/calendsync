//! Error types for integration test operations.

use thiserror::Error;

/// Result type alias for integration module.
pub type Result<T> = std::result::Result<T, IntegrationError>;

/// Errors that can occur during integration test operations.
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum IntegrationError {
    #[error("Docker is not available: {0}")]
    DockerNotAvailable(String),

    #[error("Container operation failed: {0}")]
    ContainerFailed(String),

    #[error("Container '{name}' is not healthy after {timeout_secs}s")]
    ContainerNotHealthy { name: String, timeout_secs: u64 },

    #[error("Test execution failed: {0}")]
    TestFailed(String),

    #[error("AWS SDK error: {0}")]
    AwsSdk(String),

    #[error("Table setup failed: {0}")]
    TableSetupFailed(String),

    #[error("Operation cancelled by user")]
    UserCancelled,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
