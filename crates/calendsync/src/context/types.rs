//! Pure types for request-scoped context.

use calendsync_core::calendar::User;
use uuid::Uuid;

/// Unique identifier for a request, used for tracing and logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(Uuid);

impl RequestId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Request-scoped context available to all handlers.
///
/// Complements `AppState` (application-scoped) with request-specific data.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Authenticated user (None if anonymous).
    #[cfg_attr(
        not(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock")),
        allow(dead_code)
    )]
    pub user: Option<User>,
    /// Unique request identifier for tracing.
    pub request_id: RequestId,
}
