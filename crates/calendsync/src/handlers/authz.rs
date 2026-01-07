//! Authorization helpers for API handlers.
//!
//! Provides helper functions to check calendar membership and roles.
//! Returns appropriate HTTP status codes: 403 Forbidden for authorization failures.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use calendsync_auth::AuthState;
use calendsync_core::calendar::CalendarRole;

/// Authorization error that maps to HTTP 403 Forbidden.
#[derive(Debug)]
pub enum AuthzError {
    /// User is not a member of the calendar.
    NoMembership { calendar_id: Uuid },
    /// User lacks the required permission level.
    InsufficientPermission {
        calendar_id: Uuid,
        required: &'static str,
    },
    /// Failed to look up membership (internal error).
    LookupFailed { calendar_id: Uuid, error: String },
}

impl IntoResponse for AuthzError {
    fn into_response(self) -> Response {
        match self {
            Self::NoMembership { calendar_id } => {
                tracing::warn!(calendar_id = %calendar_id, "Authorization denied: no membership");
                (StatusCode::FORBIDDEN, "No access to this calendar").into_response()
            }
            Self::InsufficientPermission {
                calendar_id,
                required,
            } => {
                tracing::warn!(
                    calendar_id = %calendar_id,
                    required = %required,
                    "Authorization denied: insufficient permission"
                );
                (
                    StatusCode::FORBIDDEN,
                    format!("Requires {required} permission"),
                )
                    .into_response()
            }
            Self::LookupFailed { calendar_id, error } => {
                tracing::error!(
                    calendar_id = %calendar_id,
                    error = %error,
                    "Authorization lookup failed"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Authorization check failed",
                )
                    .into_response()
            }
        }
    }
}

/// Requires read access (any role) to a calendar.
pub async fn require_read_access(
    auth: &AuthState,
    calendar_id: Uuid,
    user_id: Uuid,
) -> Result<CalendarRole, AuthzError> {
    let membership = auth
        .memberships
        .get_membership(calendar_id, user_id)
        .await
        .map_err(|e| AuthzError::LookupFailed {
            calendar_id,
            error: e.to_string(),
        })?;

    match membership {
        Some(m) => Ok(m.role),
        None => Err(AuthzError::NoMembership { calendar_id }),
    }
}

/// Requires write access (Owner or Writer) to a calendar.
pub async fn require_write_access(
    auth: &AuthState,
    calendar_id: Uuid,
    user_id: Uuid,
) -> Result<CalendarRole, AuthzError> {
    let membership = auth
        .memberships
        .get_membership(calendar_id, user_id)
        .await
        .map_err(|e| AuthzError::LookupFailed {
            calendar_id,
            error: e.to_string(),
        })?;

    match membership {
        Some(m) if m.role.can_write() => Ok(m.role),
        Some(_) => Err(AuthzError::InsufficientPermission {
            calendar_id,
            required: "write",
        }),
        None => Err(AuthzError::NoMembership { calendar_id }),
    }
}

/// Requires admin access (Owner only) to a calendar.
pub async fn require_admin_access(
    auth: &AuthState,
    calendar_id: Uuid,
    user_id: Uuid,
) -> Result<CalendarRole, AuthzError> {
    let membership = auth
        .memberships
        .get_membership(calendar_id, user_id)
        .await
        .map_err(|e| AuthzError::LookupFailed {
            calendar_id,
            error: e.to_string(),
        })?;

    match membership {
        Some(m) if m.role.can_administer() => Ok(m.role),
        Some(_) => Err(AuthzError::InsufficientPermission {
            calendar_id,
            required: "admin",
        }),
        None => Err(AuthzError::NoMembership { calendar_id }),
    }
}
