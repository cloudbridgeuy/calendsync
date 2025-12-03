//! Error types for DynamoDB operations.

use thiserror::Error;

/// Result type alias for dynamodb module.
pub type Result<T> = std::result::Result<T, DynamodbError>;

/// Errors that can occur during DynamoDB operations.
#[derive(Error, Debug)]
pub enum DynamodbError {
    #[error("AWS SDK error: {0}")]
    AwsSdk(String),

    #[error("Table '{table_name}' not found")]
    TableNotFound { table_name: String },

    #[error("Operation cancelled by user")]
    UserCancelled,

    #[error("Timeout waiting for table to become active")]
    TableActivationTimeout,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
