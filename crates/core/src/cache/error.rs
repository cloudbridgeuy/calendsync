use thiserror::Error;

/// Errors that can occur during cache operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CacheError {
    #[error("Cache connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Cache operation failed: {0}")]
    OperationFailed(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Publish failed: {0}")]
    PublishFailed(String),
}

/// Result type for cache operations.
pub type Result<T> = std::result::Result<T, CacheError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_failed_display() {
        let error = CacheError::ConnectionFailed("timeout".to_string());
        assert_eq!(error.to_string(), "Cache connection failed: timeout");
    }

    #[test]
    fn test_operation_failed_display() {
        let error = CacheError::OperationFailed("key not found".to_string());
        assert_eq!(error.to_string(), "Cache operation failed: key not found");
    }

    #[test]
    fn test_serialization_display() {
        let error = CacheError::Serialization("invalid JSON".to_string());
        assert_eq!(error.to_string(), "Serialization error: invalid JSON");
    }

    #[test]
    fn test_publish_failed_display() {
        let error = CacheError::PublishFailed("channel closed".to_string());
        assert_eq!(error.to_string(), "Publish failed: channel closed");
    }
}
