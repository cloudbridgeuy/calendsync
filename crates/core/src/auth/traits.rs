use async_trait::async_trait;
use url::Url;

use super::{AuthError, AuthFlowState, OidcClaims, OidcProvider, Session, SessionId};

/// Result type for auth operations.
pub type Result<T> = std::result::Result<T, AuthError>;

/// Abstraction over OIDC identity providers.
#[async_trait]
pub trait OidcProviderClient: Send + Sync {
    /// Generate authorization URL for user redirect.
    async fn authorization_url(&self, state: &str, pkce_challenge: &str) -> Result<Url>;

    /// Exchange authorization code for claims.
    async fn exchange_code(&self, code: &str, pkce_verifier: &str) -> Result<OidcClaims>;

    /// Which provider this client represents.
    fn provider(&self) -> OidcProvider;
}

/// Session storage abstraction.
#[async_trait]
pub trait SessionRepository: Send + Sync {
    /// Store a new session.
    async fn create_session(&self, session: &Session) -> Result<()>;

    /// Retrieve session by ID.
    async fn get_session(&self, id: &SessionId) -> Result<Option<Session>>;

    /// Delete a specific session.
    async fn delete_session(&self, id: &SessionId) -> Result<()>;

    /// Delete all sessions for a user (logout-all).
    async fn delete_user_sessions(&self, user_id: &str) -> Result<()>;

    /// Store PKCE/state for auth flow (short TTL).
    async fn store_auth_flow(&self, state: &str, flow: &AuthFlowState) -> Result<()>;

    /// Retrieve auth flow state without consuming it.
    /// Used to check redirect_uri before deciding whether to process the callback.
    async fn peek_auth_flow(&self, state: &str) -> Result<Option<AuthFlowState>>;

    /// Retrieve and delete auth flow state.
    async fn take_auth_flow(&self, state: &str) -> Result<Option<AuthFlowState>>;
}
