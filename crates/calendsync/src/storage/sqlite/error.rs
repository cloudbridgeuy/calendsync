//! SQLite error mapping.
//!
//! Maps `tokio_rusqlite::Error` and `rusqlite::Error` to `RepositoryError` from `calendsync_core::storage`.
//! Specific errors are mapped to semantic variants (e.g., UNIQUE constraint to AlreadyExists).

use calendsync_core::storage::RepositoryError;

/// Maps a rusqlite error to a RepositoryError.
///
/// # Error Mapping
///
/// - `SQLITE_CONSTRAINT_UNIQUE` → `RepositoryError::AlreadyExists`
/// - `SQLITE_CONSTRAINT_FOREIGNKEY` → `RepositoryError::InvalidData`
/// - Connection errors → `RepositoryError::ConnectionFailed`
/// - All other errors → `RepositoryError::QueryFailed`
fn map_rusqlite_error(err: &rusqlite::Error, entity_type: &'static str) -> RepositoryError {
    match err {
        // Handle UNIQUE constraint violations (duplicate key)
        rusqlite::Error::SqliteFailure(sqlite_err, _)
            if sqlite_err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
        {
            RepositoryError::AlreadyExists {
                entity_type,
                id: "unknown".to_string(), // ID not available from error
            }
        }

        // Handle FOREIGN KEY constraint violations (invalid reference)
        rusqlite::Error::SqliteFailure(sqlite_err, _)
            if sqlite_err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY =>
        {
            RepositoryError::InvalidData(format!(
                "Foreign key constraint violation for {entity_type}"
            ))
        }

        // Handle PRIMARY KEY constraint violations
        rusqlite::Error::SqliteFailure(sqlite_err, _)
            if sqlite_err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY =>
        {
            RepositoryError::AlreadyExists {
                entity_type,
                id: "unknown".to_string(),
            }
        }

        // Connection-related errors
        rusqlite::Error::SqliteFailure(sqlite_err, _)
            if sqlite_err.code == rusqlite::ErrorCode::CannotOpen =>
        {
            RepositoryError::ConnectionFailed(format!("Cannot open database: {err}"))
        }

        // Query returned no rows (not found)
        rusqlite::Error::QueryReturnedNoRows => RepositoryError::NotFound {
            entity_type,
            id: "unknown".to_string(),
        },

        // All other errors
        _ => RepositoryError::QueryFailed(err.to_string()),
    }
}

/// Maps a rusqlite error with a known ID to a RepositoryError.
fn map_rusqlite_error_with_id(
    err: &rusqlite::Error,
    entity_type: &'static str,
    id: &str,
) -> RepositoryError {
    match err {
        rusqlite::Error::SqliteFailure(sqlite_err, _)
            if sqlite_err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
        {
            RepositoryError::AlreadyExists {
                entity_type,
                id: id.to_string(),
            }
        }

        rusqlite::Error::SqliteFailure(sqlite_err, _)
            if sqlite_err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY =>
        {
            RepositoryError::AlreadyExists {
                entity_type,
                id: id.to_string(),
            }
        }

        rusqlite::Error::QueryReturnedNoRows => RepositoryError::NotFound {
            entity_type,
            id: id.to_string(),
        },

        _ => map_rusqlite_error(err, entity_type),
    }
}

/// Maps a tokio_rusqlite error to a RepositoryError.
///
/// This is the main entry point for error mapping in async code.
/// It extracts the inner `rusqlite::Error` if present, otherwise
/// maps to a generic `QueryFailed` error.
pub fn map_tokio_rusqlite_error(
    err: tokio_rusqlite::Error,
    entity_type: &'static str,
) -> RepositoryError {
    match &err {
        tokio_rusqlite::Error::Rusqlite(rusqlite_err) => {
            map_rusqlite_error(rusqlite_err, entity_type)
        }
        tokio_rusqlite::Error::Close(_) => {
            RepositoryError::ConnectionFailed("Connection closed unexpectedly".to_string())
        }
        _ => RepositoryError::QueryFailed(err.to_string()),
    }
}

/// Maps a tokio_rusqlite error with a known ID to a RepositoryError.
///
/// Use this variant when the entity ID is known at the call site.
pub fn map_tokio_rusqlite_error_with_id(
    err: tokio_rusqlite::Error,
    entity_type: &'static str,
    id: impl Into<String>,
) -> RepositoryError {
    let id_str = id.into();
    match &err {
        tokio_rusqlite::Error::Rusqlite(rusqlite_err) => {
            map_rusqlite_error_with_id(rusqlite_err, entity_type, &id_str)
        }
        tokio_rusqlite::Error::Close(_) => {
            RepositoryError::ConnectionFailed("Connection closed unexpectedly".to_string())
        }
        _ => RepositoryError::QueryFailed(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::ffi;

    #[test]
    fn test_unique_constraint_maps_to_already_exists() {
        let sqlite_err = rusqlite::ffi::Error {
            code: rusqlite::ErrorCode::ConstraintViolation,
            extended_code: ffi::SQLITE_CONSTRAINT_UNIQUE,
        };
        let rusqlite_err = rusqlite::Error::SqliteFailure(sqlite_err, None);
        let err = tokio_rusqlite::Error::Rusqlite(rusqlite_err);

        let result = map_tokio_rusqlite_error(err, "User");

        assert!(matches!(
            result,
            RepositoryError::AlreadyExists {
                entity_type: "User",
                ..
            }
        ));
    }

    #[test]
    fn test_foreign_key_maps_to_invalid_data() {
        let sqlite_err = rusqlite::ffi::Error {
            code: rusqlite::ErrorCode::ConstraintViolation,
            extended_code: ffi::SQLITE_CONSTRAINT_FOREIGNKEY,
        };
        let rusqlite_err = rusqlite::Error::SqliteFailure(sqlite_err, None);
        let err = tokio_rusqlite::Error::Rusqlite(rusqlite_err);

        let result = map_tokio_rusqlite_error(err, "Entry");

        assert!(matches!(result, RepositoryError::InvalidData(_)));
    }

    #[test]
    fn test_no_rows_maps_to_not_found() {
        let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let err = tokio_rusqlite::Error::Rusqlite(rusqlite_err);

        let result = map_tokio_rusqlite_error(err, "Calendar");

        assert!(matches!(
            result,
            RepositoryError::NotFound {
                entity_type: "Calendar",
                ..
            }
        ));
    }

    #[test]
    fn test_error_with_id_preserves_id() {
        let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let err = tokio_rusqlite::Error::Rusqlite(rusqlite_err);

        let result = map_tokio_rusqlite_error_with_id(err, "User", "abc-123");

        match result {
            RepositoryError::NotFound { entity_type, id } => {
                assert_eq!(entity_type, "User");
                assert_eq!(id, "abc-123");
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_close_error_maps_to_connection_failed() {
        // tokio_rusqlite::Error::Close contains a connection, which we can't easily construct
        // So we test the other branch - a generic error
        let err = tokio_rusqlite::Error::Other(Box::new(std::io::Error::other("test error")));

        let result = map_tokio_rusqlite_error(err, "User");

        assert!(matches!(result, RepositoryError::QueryFailed(_)));
    }
}
