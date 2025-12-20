use thiserror::Error;

/// Errors that can occur when constructing a date range.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DateRangeError {
    #[error("Invalid date range: start date must be before or equal to end date")]
    InvalidRange,
}

/// Errors that can occur during repository operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RepositoryError {
    #[error("{entity_type} not found: {id}")]
    NotFound {
        entity_type: &'static str,
        id: String,
    },
    #[error("{entity_type} already exists: {id}")]
    AlreadyExists {
        entity_type: &'static str,
        id: String,
    },
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Query failed: {0}")]
    QueryFailed(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

/// Result type for repository operations.
pub type Result<T> = std::result::Result<T, RepositoryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_range_error_display() {
        assert_eq!(
            DateRangeError::InvalidRange.to_string(),
            "Invalid date range: start date must be before or equal to end date"
        );
    }

    #[test]
    fn test_repository_error_not_found_display() {
        let error = RepositoryError::NotFound {
            entity_type: "CalendarEntry",
            id: "abc-123".to_string(),
        };
        assert_eq!(error.to_string(), "CalendarEntry not found: abc-123");
    }

    #[test]
    fn test_repository_error_already_exists_display() {
        let error = RepositoryError::AlreadyExists {
            entity_type: "Calendar",
            id: "my-calendar".to_string(),
        };
        assert_eq!(error.to_string(), "Calendar already exists: my-calendar");
    }

    #[test]
    fn test_repository_error_connection_failed_display() {
        let error = RepositoryError::ConnectionFailed("timeout after 30s".to_string());
        assert_eq!(error.to_string(), "Connection failed: timeout after 30s");
    }

    #[test]
    fn test_repository_error_query_failed_display() {
        let error = RepositoryError::QueryFailed("invalid partition key".to_string());
        assert_eq!(error.to_string(), "Query failed: invalid partition key");
    }

    #[test]
    fn test_repository_error_serialization_display() {
        let error = RepositoryError::Serialization("missing required field".to_string());
        assert_eq!(
            error.to_string(),
            "Serialization error: missing required field"
        );
    }

    #[test]
    fn test_repository_error_invalid_data_display() {
        let error = RepositoryError::InvalidData("date out of range".to_string());
        assert_eq!(error.to_string(), "Invalid data: date out of range");
    }
}
