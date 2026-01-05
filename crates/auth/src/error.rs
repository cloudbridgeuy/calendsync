use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

/// Auth errors for the calendsync_auth crate.
///
/// This wraps the core `AuthError` and adds crate-specific error variants
/// for I/O operations that can't be in the functional core.
#[derive(Debug, Error)]
pub enum AuthError {
    /// Error from the core auth module (validation, token parsing, etc.)
    #[error(transparent)]
    Core(#[from] calendsync_core::auth::AuthError),

    /// HTTP client error during OIDC flow
    #[error("HTTP error: {0}")]
    Http(String),

    /// Configuration error
    #[error("configuration error: {0}")]
    Config(String),

    /// Provider not configured
    #[error("provider not configured: {0}")]
    ProviderNotConfigured(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        use calendsync_core::auth::AuthError as CoreError;

        let (status, message) = match &self {
            AuthError::Core(core_err) => match core_err {
                CoreError::InvalidState | CoreError::PkceNotFound => {
                    (StatusCode::BAD_REQUEST, self.to_string())
                }
                CoreError::SessionNotFound | CoreError::SessionExpired => {
                    (StatusCode::UNAUTHORIZED, self.to_string())
                }
                CoreError::InvalidToken(_) | CoreError::MissingClaim(_) => {
                    (StatusCode::UNAUTHORIZED, self.to_string())
                }
                CoreError::CodeExchange(_) | CoreError::Storage(_) | CoreError::Provider(_) => {
                    tracing::error!("Auth error: {}", self);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal server error".to_string(),
                    )
                }
            },
            AuthError::Http(_) => {
                tracing::error!("HTTP error during auth: {}", self);
                (
                    StatusCode::BAD_GATEWAY,
                    "Authentication provider error".to_string(),
                )
            }
            AuthError::Config(_) => {
                tracing::error!("Config error: {}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Server configuration error".to_string(),
                )
            }
            AuthError::ProviderNotConfigured(provider) => (
                StatusCode::NOT_FOUND,
                format!("Authentication provider '{}' is not configured", provider),
            ),
        };

        (status, message).into_response()
    }
}
