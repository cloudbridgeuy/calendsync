use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid OIDC state parameter")]
    InvalidState,

    #[error("PKCE verifier not found for state")]
    PkceNotFound,

    #[error("failed to exchange authorization code: {0}")]
    CodeExchange(String),

    #[error("invalid ID token: {0}")]
    InvalidToken(String),

    #[error("missing required claim: {0}")]
    MissingClaim(String),

    #[error("session not found")]
    SessionNotFound,

    #[error("session expired")]
    SessionExpired,

    #[error("storage error: {0}")]
    Storage(String),

    #[error("provider error: {0}")]
    Provider(String),
}
