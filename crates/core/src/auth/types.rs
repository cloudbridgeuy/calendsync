use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Cryptographically random session identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Supported OIDC providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OidcProvider {
    Google,
    Apple,
}

impl std::fmt::Display for OidcProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Google => write!(f, "google"),
            Self::Apple => write!(f, "apple"),
        }
    }
}

/// Authenticated user session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub user_id: String,
    pub provider: OidcProvider,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Provider-agnostic claims extracted from OIDC ID token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcClaims {
    /// Provider's unique user identifier.
    pub subject: String,
    /// User's email address.
    pub email: Option<String>,
    /// User's display name.
    pub name: Option<String>,
    /// Which provider issued these claims.
    pub provider: OidcProvider,
}

/// PKCE and state data stored during auth flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthFlowState {
    pub pkce_verifier: String,
    pub provider: OidcProvider,
    pub created_at: DateTime<Utc>,
    /// URL to redirect to after successful authentication.
    pub return_to: Option<String>,
    /// Custom redirect URI for native apps (e.g., calendsync://auth/callback).
    /// When set, the callback will redirect to this URI with code+state params
    /// instead of processing the code exchange (native app calls /auth/exchange).
    pub redirect_uri: Option<String>,
}
