//! Pure functions for mapping repository errors to HTTP status codes.
//!
//! This module provides HTTP status code mappings for [`RepositoryError`] variants,
//! following the Functional Core pattern - pure functions with no side effects.

use super::RepositoryError;

/// Maps a [`RepositoryError`] to an HTTP status code.
///
/// This is a pure function that returns the appropriate HTTP status code
/// for each error variant:
///
/// - `NotFound` -> 404 (Not Found)
/// - `AlreadyExists` -> 409 (Conflict)
/// - `ConnectionFailed` -> 503 (Service Unavailable)
/// - `QueryFailed` -> 500 (Internal Server Error)
/// - `Serialization` -> 500 (Internal Server Error)
/// - `InvalidData` -> 400 (Bad Request)
///
/// # Examples
///
/// ```
/// use calendsync_core::storage::{RepositoryError, repository_error_to_status_code};
///
/// let error = RepositoryError::NotFound {
///     entity_type: "Calendar",
///     id: "abc-123".to_string(),
/// };
/// assert_eq!(repository_error_to_status_code(&error), 404);
/// ```
pub fn repository_error_to_status_code(error: &RepositoryError) -> u16 {
    match error {
        RepositoryError::NotFound { .. } => 404,
        RepositoryError::AlreadyExists { .. } => 409,
        RepositoryError::ConnectionFailed(_) => 503,
        RepositoryError::QueryFailed(_) => 500,
        RepositoryError::Serialization(_) => 500,
        RepositoryError::InvalidData(_) => 400,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_maps_to_404() {
        let error = RepositoryError::NotFound {
            entity_type: "Calendar",
            id: "cal-123".to_string(),
        };
        assert_eq!(repository_error_to_status_code(&error), 404);
    }

    #[test]
    fn test_already_exists_maps_to_409() {
        let error = RepositoryError::AlreadyExists {
            entity_type: "CalendarEntry",
            id: "entry-456".to_string(),
        };
        assert_eq!(repository_error_to_status_code(&error), 409);
    }

    #[test]
    fn test_connection_failed_maps_to_503() {
        let error = RepositoryError::ConnectionFailed("database connection timeout".to_string());
        assert_eq!(repository_error_to_status_code(&error), 503);
    }

    #[test]
    fn test_query_failed_maps_to_500() {
        let error = RepositoryError::QueryFailed("invalid query syntax".to_string());
        assert_eq!(repository_error_to_status_code(&error), 500);
    }

    #[test]
    fn test_serialization_maps_to_500() {
        let error = RepositoryError::Serialization("failed to deserialize JSON".to_string());
        assert_eq!(repository_error_to_status_code(&error), 500);
    }

    #[test]
    fn test_invalid_data_maps_to_400() {
        let error = RepositoryError::InvalidData("date format is invalid".to_string());
        assert_eq!(repository_error_to_status_code(&error), 400);
    }
}
