# OIDC Authentication Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Add OIDC authentication with Google and Apple providers, server-side sessions, and role-based calendar access control.

**Architecture:** Library-only approach using `openidconnect` crate. New `calendsync_auth` crate handles OIDC flows and session storage (Imperative Shell). Pure auth types and traits live in `calendsync_core` under `auth` feature (Functional Core). Sessions stored in SQLite (dev) or Redis (prod) via feature-gated `SessionStore`. Mock IdP server for development testing.

**Tech Stack:** `openidconnect`, `oauth2`, axum extractors, secure cookies, PKCE flow, SQLite/Redis sessions.

**Domain:** `calendsync.app`

---

## Phase 1: Core Types and Traits

### Task 1.1: Add auth module to calendsync_core

**File:** `crates/core/Cargo.toml`

Add auth feature flag:

```toml
[features]
default = []
auth = []
```

**File:** `crates/core/src/lib.rs`

Add conditional auth module:

```rust
#[cfg(feature = "auth")]
pub mod auth;
```

**File:** `crates/core/src/auth/mod.rs`

```rust
mod error;
mod traits;
mod types;

pub use error::AuthError;
pub use traits::{OidcProviderClient, SessionRepository};
pub use types::{OidcClaims, OidcProvider, Session, SessionId};
```

---

### Task 1.2: Define auth types

**File:** `crates/core/src/auth/types.rs`

```rust
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
}
```

---

### Task 1.3: Define auth traits

**File:** `crates/core/src/auth/traits.rs`

```rust
use async_trait::async_trait;
use url::Url;

use super::{AuthError, AuthFlowState, OidcClaims, OidcProvider, Session, SessionId};

/// Result type for auth operations.
pub type Result<T> = std::result::Result<T, AuthError>;

/// Abstraction over OIDC identity providers.
#[async_trait]
pub trait OidcProviderClient: Send + Sync {
    /// Generate authorization URL for user redirect.
    async fn authorization_url(
        &self,
        state: &str,
        pkce_challenge: &str,
    ) -> Result<Url>;

    /// Exchange authorization code for claims.
    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<OidcClaims>;

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

    /// Retrieve and delete auth flow state.
    async fn take_auth_flow(&self, state: &str) -> Result<Option<AuthFlowState>>;
}
```

---

### Task 1.4: Define auth errors

**File:** `crates/core/src/auth/error.rs`

```rust
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
```

---

### Task 1.5: Add pure auth functions

**File:** `crates/core/src/auth/functions.rs`

```rust
use chrono::{DateTime, Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};

use super::{Session, SessionId};

/// Generate a cryptographically random session ID.
pub fn generate_session_id() -> SessionId {
    let id: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    SessionId::new(id)
}

/// Generate a random state parameter for CSRF protection.
pub fn generate_state() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

/// Check if a session has expired.
pub fn is_session_expired(session: &Session, now: DateTime<Utc>) -> bool {
    session.expires_at <= now
}

/// Calculate session expiry from creation time and TTL.
pub fn calculate_expiry(created_at: DateTime<Utc>, ttl: Duration) -> DateTime<Utc> {
    created_at + ttl
}

/// Extract username from email if no name provided.
pub fn email_to_name(email: &str) -> String {
    email
        .split('@')
        .next()
        .unwrap_or("User")
        .to_string()
}
```

Update `crates/core/src/auth/mod.rs`:

```rust
mod error;
mod functions;
mod traits;
mod types;

pub use error::AuthError;
pub use functions::{
    calculate_expiry, email_to_name, generate_session_id, generate_state, is_session_expired,
};
pub use traits::{OidcProviderClient, Result, SessionRepository};
pub use types::{AuthFlowState, OidcClaims, OidcProvider, Session, SessionId};
```

---

### Task 1.6: Add provider fields to User type

**File:** `crates/core/src/calendar/types.rs`

Add fields to existing `User` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    // New fields:
    pub provider: Option<String>,         // "google" or "apple"
    pub provider_subject: Option<String>, // Provider's unique user ID
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

---

### Task 1.7: Add is_default field to Calendar type

**File:** `crates/core/src/calendar/types.rs`

Add field to existing `Calendar` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String,
    pub name: String,
    pub is_default: bool, // New: true for user's undeletable personal calendar
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

---

### Task 1.8: Write unit tests for pure auth functions

**File:** `crates/core/src/auth/functions_test.rs`

```rust
#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::*;

    #[test]
    fn generate_session_id_produces_32_char_alphanumeric() {
        let id = generate_session_id();
        assert_eq!(id.as_str().len(), 32);
        assert!(id.as_str().chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn generate_session_id_is_unique() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn generate_state_produces_32_char_string() {
        let state = generate_state();
        assert_eq!(state.len(), 32);
    }

    #[test]
    fn is_session_expired_returns_false_for_future_expiry() {
        let now = Utc::now();
        let session = Session {
            id: generate_session_id(),
            user_id: "user-1".to_string(),
            provider: OidcProvider::Google,
            created_at: now,
            expires_at: now + Duration::hours(1),
        };
        assert!(!is_session_expired(&session, now));
    }

    #[test]
    fn is_session_expired_returns_true_for_past_expiry() {
        let now = Utc::now();
        let session = Session {
            id: generate_session_id(),
            user_id: "user-1".to_string(),
            provider: OidcProvider::Google,
            created_at: now - Duration::hours(2),
            expires_at: now - Duration::hours(1),
        };
        assert!(is_session_expired(&session, now));
    }

    #[test]
    fn is_session_expired_returns_true_at_exact_expiry() {
        let now = Utc::now();
        let session = Session {
            id: generate_session_id(),
            user_id: "user-1".to_string(),
            provider: OidcProvider::Google,
            created_at: now - Duration::hours(1),
            expires_at: now,
        };
        assert!(is_session_expired(&session, now));
    }

    #[test]
    fn calculate_expiry_adds_ttl_to_created_at() {
        let created = Utc::now();
        let ttl = Duration::days(7);
        let expiry = calculate_expiry(created, ttl);
        assert_eq!(expiry, created + ttl);
    }

    #[test]
    fn email_to_name_extracts_username() {
        assert_eq!(email_to_name("john.doe@example.com"), "john.doe");
        assert_eq!(email_to_name("alice@test.org"), "alice");
    }

    #[test]
    fn email_to_name_handles_invalid_email() {
        assert_eq!(email_to_name("no-at-sign"), "no-at-sign");
        assert_eq!(email_to_name(""), "User");
    }
}
```

**Run tests:**

```bash
cargo test -p calendsync_core --features auth
```

---

## Phase 2: Create calendsync_auth Crate

### Task 2.1: Create crate structure

**File:** `crates/auth/Cargo.toml`

```toml
[package]
name = "calendsync_auth"
version = "0.1.0"
edition = "2021"

[features]
default = []
sqlite = ["sqlx"]
redis = ["fred"]
mock = []

[dependencies]
calendsync_core = { path = "../core", features = ["auth"] }

# Async
async-trait = { workspace = true }
tokio = { workspace = true }

# Web
axum = { workspace = true }
axum-extra = { workspace = true, features = ["cookie"] }
tower = { workspace = true }
tower-cookies = { workspace = true }

# OIDC
openidconnect = "3"
oauth2 = "4"
url = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Time
chrono = { workspace = true }

# Errors
thiserror = { workspace = true }

# Logging
tracing = { workspace = true }

# Storage (feature-gated)
sqlx = { workspace = true, features = ["sqlite", "runtime-tokio"], optional = true }
fred = { workspace = true, optional = true }
```

**File:** `crates/auth/src/lib.rs`

```rust
mod config;
mod error;
mod extractors;
mod handlers;
mod providers;
mod sessions;
mod state;

pub use config::AuthConfig;
pub use error::AuthError;
pub use extractors::{CurrentUser, OptionalUser};
pub use handlers::auth_routes;
pub use sessions::SessionStore;
pub use state::AuthState;

#[cfg(feature = "mock")]
pub mod mock_idp;
```

---

### Task 2.2: Define AuthConfig

**File:** `crates/auth/src/config.rs`

```rust
use std::time::Duration;

use url::Url;

/// Configuration for a single OIDC provider.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uri: Url,
}

/// Apple-specific configuration (uses signed JWT for client secret).
#[derive(Debug, Clone)]
pub struct AppleConfig {
    pub client_id: String,
    pub team_id: String,
    pub key_id: String,
    pub private_key: String, // PEM-encoded ES256 private key
    pub redirect_uri: Url,
}

/// Complete auth configuration.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub google: Option<ProviderConfig>,
    pub apple: Option<AppleConfig>,
    pub session_ttl: Duration,
    pub base_url: Url,
    pub cookie_name: String,
    pub cookie_secure: bool,
}

impl AuthConfig {
    /// Load from environment variables.
    pub fn from_env() -> Result<Self, std::env::VarError> {
        let base_url: Url = std::env::var("AUTH_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string())
            .parse()
            .expect("AUTH_BASE_URL must be valid URL");

        let google = match std::env::var("GOOGLE_CLIENT_ID") {
            Ok(client_id) => Some(ProviderConfig {
                client_id,
                client_secret: Some(std::env::var("GOOGLE_CLIENT_SECRET")?),
                redirect_uri: base_url.join("/auth/google/callback").unwrap(),
            }),
            Err(_) => None,
        };

        let apple = match std::env::var("APPLE_CLIENT_ID") {
            Ok(client_id) => Some(AppleConfig {
                client_id,
                team_id: std::env::var("APPLE_TEAM_ID")?,
                key_id: std::env::var("APPLE_KEY_ID")?,
                private_key: std::env::var("APPLE_PRIVATE_KEY")?,
                redirect_uri: base_url.join("/auth/apple/callback").unwrap(),
            }),
            Err(_) => None,
        };

        let session_ttl = std::env::var("SESSION_TTL_DAYS")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(7 * 24 * 60 * 60)); // 7 days default

        let cookie_secure = std::env::var("COOKIE_SECURE")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

        Ok(Self {
            google,
            apple,
            session_ttl,
            base_url,
            cookie_name: "session".to_string(),
            cookie_secure,
        })
    }
}
```

---

### Task 2.3: Implement Google provider

**File:** `crates/auth/src/providers/mod.rs`

```rust
mod google;
#[cfg(feature = "mock")]
mod mock;

pub use google::GoogleProvider;
#[cfg(feature = "mock")]
pub use mock::MockProvider;
```

**File:** `crates/auth/src/providers/google.rs`

```rust
use async_trait::async_trait;
use calendsync_core::auth::{AuthError, OidcClaims, OidcProvider, OidcProviderClient, Result};
use openidconnect::{
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
    reqwest::async_http_client,
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
};
use url::Url;

use crate::config::ProviderConfig;

pub struct GoogleProvider {
    client: CoreClient,
}

impl GoogleProvider {
    pub async fn new(config: &ProviderConfig) -> Result<Self> {
        let issuer_url = IssuerUrl::new("https://accounts.google.com".to_string())
            .map_err(|e| AuthError::Provider(e.to_string()))?;

        let provider_metadata =
            CoreProviderMetadata::discover_async(issuer_url, async_http_client)
                .await
                .map_err(|e| AuthError::Provider(e.to_string()))?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(config.client_id.clone()),
            config.client_secret.clone().map(ClientSecret::new),
        )
        .set_redirect_uri(
            RedirectUrl::new(config.redirect_uri.to_string())
                .map_err(|e| AuthError::Provider(e.to_string()))?,
        );

        Ok(Self { client })
    }
}

#[async_trait]
impl OidcProviderClient for GoogleProvider {
    async fn authorization_url(&self, state: &str, pkce_challenge: &str) -> Result<Url> {
        let (auth_url, _, _) = self
            .client
            .authorize_url(
                || CsrfToken::new(state.to_string()),
                || Nonce::new(calendsync_core::auth::generate_state()),
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .set_pkce_challenge(PkceCodeChallenge::new(
                pkce_challenge.to_string(),
                openidconnect::PkceCodeChallengeMethod::S256,
            ))
            .url();

        Ok(auth_url.into())
    }

    async fn exchange_code(&self, code: &str, pkce_verifier: &str) -> Result<OidcClaims> {
        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| AuthError::CodeExchange(e.to_string()))?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| AuthError::InvalidToken("No ID token in response".to_string()))?;

        let claims = id_token.claims(&self.client.id_token_verifier(), |_| Ok(()))
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(OidcClaims {
            subject: claims.subject().to_string(),
            email: claims.email().map(|e| e.to_string()),
            name: claims
                .name()
                .and_then(|n| n.get(None))
                .map(|n| n.to_string()),
            provider: OidcProvider::Google,
        })
    }

    fn provider(&self) -> OidcProvider {
        OidcProvider::Google
    }
}
```

---

### Task 2.4: Implement Apple provider

**File:** `crates/auth/src/providers/apple.rs`

```rust
use async_trait::async_trait;
use calendsync_core::auth::{AuthError, OidcClaims, OidcProvider, OidcProviderClient, Result};
use openidconnect::{
    core::{CoreClient, CoreProviderMetadata},
    reqwest::async_http_client,
    AuthorizationCode, ClientId, CsrfToken, IssuerUrl, Nonce,
    OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
};
use url::Url;

use crate::config::AppleConfig;

pub struct AppleProvider {
    client: CoreClient,
    config: AppleConfig,
}

impl AppleProvider {
    pub async fn new(config: &AppleConfig) -> Result<Self> {
        let issuer_url = IssuerUrl::new("https://appleid.apple.com".to_string())
            .map_err(|e| AuthError::Provider(e.to_string()))?;

        let provider_metadata =
            CoreProviderMetadata::discover_async(issuer_url, async_http_client)
                .await
                .map_err(|e| AuthError::Provider(e.to_string()))?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(config.client_id.clone()),
            None, // Apple uses signed JWT, set per-request
        )
        .set_redirect_uri(
            RedirectUrl::new(config.redirect_uri.to_string())
                .map_err(|e| AuthError::Provider(e.to_string()))?,
        );

        Ok(Self {
            client,
            config: config.clone(),
        })
    }

    /// Generate Apple client secret (signed JWT).
    fn generate_client_secret(&self) -> Result<String> {
        // Apple requires a signed JWT as client_secret
        // See: https://developer.apple.com/documentation/sign_in_with_apple/generate_and_validate_tokens
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        use serde::Serialize;
        use std::time::{SystemTime, UNIX_EPOCH};

        #[derive(Serialize)]
        struct Claims {
            iss: String,
            iat: u64,
            exp: u64,
            aud: String,
            sub: String,
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            iss: self.config.team_id.clone(),
            iat: now,
            exp: now + 86400 * 180, // 180 days max
            aud: "https://appleid.apple.com".to_string(),
            sub: self.config.client_id.clone(),
        };

        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.config.key_id.clone());

        let key = EncodingKey::from_ec_pem(self.config.private_key.as_bytes())
            .map_err(|e| AuthError::Provider(format!("Invalid Apple private key: {}", e)))?;

        encode(&header, &claims, &key)
            .map_err(|e| AuthError::Provider(format!("Failed to sign Apple JWT: {}", e)))
    }
}

#[async_trait]
impl OidcProviderClient for AppleProvider {
    async fn authorization_url(&self, state: &str, pkce_challenge: &str) -> Result<Url> {
        let (auth_url, _, _) = self
            .client
            .authorize_url(
                || CsrfToken::new(state.to_string()),
                || Nonce::new(calendsync_core::auth::generate_state()),
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("name".to_string()))
            .set_pkce_challenge(PkceCodeChallenge::new(
                pkce_challenge.to_string(),
                openidconnect::PkceCodeChallengeMethod::S256,
            ))
            .add_extra_param("response_mode", "form_post") // Apple requires this
            .url();

        Ok(auth_url.into())
    }

    async fn exchange_code(&self, code: &str, pkce_verifier: &str) -> Result<OidcClaims> {
        let client_secret = self.generate_client_secret()?;

        // Apple requires setting client_secret per-request
        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.to_string()))
            .add_extra_param("client_secret", &client_secret)
            .request_async(async_http_client)
            .await
            .map_err(|e| AuthError::CodeExchange(e.to_string()))?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| AuthError::InvalidToken("No ID token in response".to_string()))?;

        let claims = id_token
            .claims(&self.client.id_token_verifier(), |_| Ok(()))
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(OidcClaims {
            subject: claims.subject().to_string(),
            email: claims.email().map(|e| e.to_string()),
            name: claims
                .name()
                .and_then(|n| n.get(None))
                .map(|n| n.to_string()),
            provider: OidcProvider::Apple,
        })
    }

    fn provider(&self) -> OidcProvider {
        OidcProvider::Apple
    }
}
```

Add `jsonwebtoken` dependency to `crates/auth/Cargo.toml`:

```toml
jsonwebtoken = "9"
```

---

### Task 2.5: Implement SQLite SessionStore

**File:** `crates/auth/src/sessions/mod.rs`

```rust
#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(feature = "redis")]
mod redis_impl;

#[cfg(feature = "sqlite")]
pub use sqlite::SessionStore;
#[cfg(feature = "redis")]
pub use redis_impl::SessionStore;
```

**File:** `crates/auth/src/sessions/sqlite.rs`

```rust
use async_trait::async_trait;
use calendsync_core::auth::{
    AuthError, AuthFlowState, OidcProvider, Result, Session, SessionId, SessionRepository,
};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

pub struct SessionStore {
    pool: SqlitePool,
}

impl SessionStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

            CREATE TABLE IF NOT EXISTS auth_flows (
                state TEXT PRIMARY KEY,
                pkce_verifier TEXT NOT NULL,
                provider TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl SessionRepository for SessionStore {
    async fn create_session(&self, session: &Session) -> Result<()> {
        sqlx::query(
            "INSERT INTO sessions (id, user_id, provider, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(session.id.as_str())
        .bind(&session.user_id)
        .bind(session.provider.to_string())
        .bind(session.created_at.to_rfc3339())
        .bind(session.expires_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn get_session(&self, id: &SessionId) -> Result<Option<Session>> {
        let row = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, user_id, provider, created_at, expires_at FROM sessions WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

        match row {
            Some((id, user_id, provider, created_at, expires_at)) => {
                let provider = match provider.as_str() {
                    "google" => OidcProvider::Google,
                    "apple" => OidcProvider::Apple,
                    _ => return Err(AuthError::Storage(format!("Unknown provider: {}", provider))),
                };

                Ok(Some(Session {
                    id: SessionId::new(id),
                    user_id,
                    provider,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map_err(|e| AuthError::Storage(e.to_string()))?
                        .with_timezone(&Utc),
                    expires_at: DateTime::parse_from_rfc3339(&expires_at)
                        .map_err(|e| AuthError::Storage(e.to_string()))?
                        .with_timezone(&Utc),
                }))
            }
            None => Ok(None),
        }
    }

    async fn delete_session(&self, id: &SessionId) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn delete_user_sessions(&self, user_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn store_auth_flow(&self, state: &str, flow: &AuthFlowState) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO auth_flows (state, pkce_verifier, provider, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(state)
        .bind(&flow.pkce_verifier)
        .bind(flow.provider.to_string())
        .bind(flow.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn take_auth_flow(&self, state: &str) -> Result<Option<AuthFlowState>> {
        let row = sqlx::query_as::<_, (String, String, String)>(
            "SELECT pkce_verifier, provider, created_at FROM auth_flows WHERE state = ?",
        )
        .bind(state)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

        if row.is_some() {
            sqlx::query("DELETE FROM auth_flows WHERE state = ?")
                .bind(state)
                .execute(&self.pool)
                .await
                .map_err(|e| AuthError::Storage(e.to_string()))?;
        }

        match row {
            Some((pkce_verifier, provider, created_at)) => {
                let provider = match provider.as_str() {
                    "google" => OidcProvider::Google,
                    "apple" => OidcProvider::Apple,
                    _ => return Err(AuthError::Storage(format!("Unknown provider: {}", provider))),
                };

                Ok(Some(AuthFlowState {
                    pkce_verifier,
                    provider,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map_err(|e| AuthError::Storage(e.to_string()))?
                        .with_timezone(&Utc),
                }))
            }
            None => Ok(None),
        }
    }
}
```

---

### Task 2.6: Implement Redis SessionStore

**File:** `crates/auth/src/sessions/redis_impl.rs`

```rust
use async_trait::async_trait;
use calendsync_core::auth::{
    AuthError, AuthFlowState, Result, Session, SessionId, SessionRepository,
};
use fred::prelude::*;
use std::time::Duration;

pub struct SessionStore {
    pool: RedisPool,
    session_ttl: Duration,
    flow_ttl: Duration,
}

impl SessionStore {
    pub fn new(pool: RedisPool, session_ttl: Duration) -> Self {
        Self {
            pool,
            session_ttl,
            flow_ttl: Duration::from_secs(600), // 10 minutes for auth flow
        }
    }

    fn session_key(id: &SessionId) -> String {
        format!("session:{}", id)
    }

    fn user_sessions_key(user_id: &str) -> String {
        format!("user_sessions:{}", user_id)
    }

    fn flow_key(state: &str) -> String {
        format!("auth_flow:{}", state)
    }
}

#[async_trait]
impl SessionRepository for SessionStore {
    async fn create_session(&self, session: &Session) -> Result<()> {
        let key = Self::session_key(&session.id);
        let value = serde_json::to_string(session)
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        let ttl_secs = self.session_ttl.as_secs() as i64;

        self.pool
            .set::<(), _, _>(&key, &value, Some(Expiration::EX(ttl_secs)), None, false)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Track session in user's session set
        let user_key = Self::user_sessions_key(&session.user_id);
        self.pool
            .sadd::<(), _, _>(&user_key, session.id.as_str())
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn get_session(&self, id: &SessionId) -> Result<Option<Session>> {
        let key = Self::session_key(id);
        let value: Option<String> = self
            .pool
            .get(&key)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        match value {
            Some(json) => {
                let session: Session = serde_json::from_str(&json)
                    .map_err(|e| AuthError::Storage(e.to_string()))?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    async fn delete_session(&self, id: &SessionId) -> Result<()> {
        // Get session first to find user_id
        if let Some(session) = self.get_session(id).await? {
            let key = Self::session_key(id);
            self.pool
                .del::<(), _>(&key)
                .await
                .map_err(|e| AuthError::Storage(e.to_string()))?;

            // Remove from user's session set
            let user_key = Self::user_sessions_key(&session.user_id);
            self.pool
                .srem::<(), _, _>(&user_key, id.as_str())
                .await
                .map_err(|e| AuthError::Storage(e.to_string()))?;
        }

        Ok(())
    }

    async fn delete_user_sessions(&self, user_id: &str) -> Result<()> {
        let user_key = Self::user_sessions_key(user_id);

        // Get all session IDs for user
        let session_ids: Vec<String> = self
            .pool
            .smembers(&user_key)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Delete each session
        for id in &session_ids {
            let key = format!("session:{}", id);
            self.pool
                .del::<(), _>(&key)
                .await
                .map_err(|e| AuthError::Storage(e.to_string()))?;
        }

        // Delete the user sessions set
        self.pool
            .del::<(), _>(&user_key)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn store_auth_flow(&self, state: &str, flow: &AuthFlowState) -> Result<()> {
        let key = Self::flow_key(state);
        let value = serde_json::to_string(flow)
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        let ttl_secs = self.flow_ttl.as_secs() as i64;

        self.pool
            .set::<(), _, _>(&key, &value, Some(Expiration::EX(ttl_secs)), None, false)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn take_auth_flow(&self, state: &str) -> Result<Option<AuthFlowState>> {
        let key = Self::flow_key(state);

        // Get and delete atomically
        let value: Option<String> = self
            .pool
            .getdel(&key)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        match value {
            Some(json) => {
                let flow: AuthFlowState = serde_json::from_str(&json)
                    .map_err(|e| AuthError::Storage(e.to_string()))?;
                Ok(Some(flow))
            }
            None => Ok(None),
        }
    }
}
```

---

### Task 2.7: Implement CurrentUser extractor

**File:** `crates/auth/src/extractors.rs`

```rust
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};
use axum_extra::extract::CookieJar;
use calendsync_core::auth::{is_session_expired, SessionId, SessionRepository};
use calendsync_core::calendar::User;
use calendsync_core::storage::UserRepository;
use chrono::Utc;
use std::sync::Arc;

use crate::{AuthConfig, AuthError, AuthState};

/// Extractor for authenticated user. Returns 401 if not authenticated.
pub struct CurrentUser(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    AuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_state = AuthState::from_ref(state);

        // Try Authorization header first (for API/mobile clients)
        let session_id = if let Some(auth_header) = parts.headers.get(AUTHORIZATION) {
            let header_value = auth_header
                .to_str()
                .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid authorization header"))?;

            if let Some(token) = header_value.strip_prefix("Bearer ") {
                Some(SessionId::new(token.to_string()))
            } else {
                None
            }
        } else {
            None
        };

        // Fall back to cookie (for web clients)
        let session_id = match session_id {
            Some(id) => id,
            None => {
                let jar = CookieJar::from_headers(&parts.headers);
                let cookie = jar
                    .get(&auth_state.config.cookie_name)
                    .ok_or((StatusCode::UNAUTHORIZED, "No session cookie"))?;

                SessionId::new(cookie.value().to_string())
            }
        };

        // Look up session
        let session = auth_state
            .sessions
            .get_session(&session_id)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Session lookup failed"))?
            .ok_or((StatusCode::UNAUTHORIZED, "Session not found"))?;

        // Check expiry
        if is_session_expired(&session, Utc::now()) {
            return Err((StatusCode::UNAUTHORIZED, "Session expired"));
        }

        // Look up user
        let user = auth_state
            .users
            .get_user(&session.user_id)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "User lookup failed"))?
            .ok_or((StatusCode::UNAUTHORIZED, "User not found"))?;

        Ok(CurrentUser(user))
    }
}

/// Extractor for optionally authenticated user. Returns None if not authenticated.
pub struct OptionalUser(pub Option<User>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalUser
where
    AuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match CurrentUser::from_request_parts(parts, state).await {
            Ok(CurrentUser(user)) => Ok(OptionalUser(Some(user))),
            Err(_) => Ok(OptionalUser(None)),
        }
    }
}
```

---

### Task 2.8: Implement auth handlers

**File:** `crates/auth/src/handlers.rs`

```rust
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Form, Json, Router,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use calendsync_core::auth::{
    calculate_expiry, generate_session_id, generate_state, AuthFlowState, OidcClaims,
    OidcProvider, OidcProviderClient, Session, SessionRepository,
};
use calendsync_core::calendar::{Calendar, CalendarMembership, CalendarRole, User};
use calendsync_core::storage::{CalendarRepository, MembershipRepository, UserRepository};
use chrono::{Duration, Utc};
use openidconnect::{PkceCodeChallenge, PkceCodeVerifier};
use serde::Deserialize;

use crate::{AuthConfig, AuthError, AuthState};

/// Query parameters for OAuth callback.
#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

/// Apple sends callback as POST with form data.
#[derive(Deserialize)]
pub struct AppleCallbackForm {
    pub code: String,
    pub state: String,
    pub user: Option<String>, // JSON string with name on first login
}

pub fn auth_routes<S>() -> Router<S>
where
    AuthState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/auth/google/login", get(google_login))
        .route("/auth/google/callback", get(google_callback))
        .route("/auth/apple/login", get(apple_login))
        .route("/auth/apple/callback", post(apple_callback))
        .route("/auth/logout", post(logout))
        .route("/auth/logout-all", post(logout_all))
        .route("/auth/me", get(me))
}

async fn google_login(State(state): State<AuthState>) -> Result<Redirect, AuthError> {
    initiate_login(&state, OidcProvider::Google).await
}

async fn apple_login(State(state): State<AuthState>) -> Result<Redirect, AuthError> {
    initiate_login(&state, OidcProvider::Apple).await
}

async fn initiate_login(state: &AuthState, provider: OidcProvider) -> Result<Redirect, AuthError> {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let csrf_state = generate_state();

    // Store PKCE verifier for callback
    let flow = AuthFlowState {
        pkce_verifier: pkce_verifier.secret().to_string(),
        provider,
        created_at: Utc::now(),
    };
    state.sessions.store_auth_flow(&csrf_state, &flow).await?;

    // Get provider client and generate auth URL
    let provider_client = state.get_provider(provider)?;
    let auth_url = provider_client
        .authorization_url(&csrf_state, pkce_challenge.as_str())
        .await?;

    Ok(Redirect::to(auth_url.as_str()))
}

async fn google_callback(
    State(state): State<AuthState>,
    Query(params): Query<CallbackQuery>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AuthError> {
    handle_callback(&state, &params.code, &params.state, jar, None).await
}

async fn apple_callback(
    State(state): State<AuthState>,
    Form(form): Form<AppleCallbackForm>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AuthError> {
    // Apple includes user info in form on first login
    let user_info = form.user.as_ref().and_then(|u| {
        serde_json::from_str::<serde_json::Value>(u).ok()
    });

    let name = user_info.as_ref().and_then(|u| {
        let first = u.get("name")?.get("firstName")?.as_str()?;
        let last = u.get("name")?.get("lastName")?.as_str()?;
        Some(format!("{} {}", first, last))
    });

    handle_callback(&state, &form.code, &form.state, jar, name).await
}

async fn handle_callback(
    state: &AuthState,
    code: &str,
    csrf_state: &str,
    jar: CookieJar,
    apple_name: Option<String>,
) -> Result<(CookieJar, Redirect), AuthError> {
    // Retrieve and validate PKCE verifier
    let flow = state
        .sessions
        .take_auth_flow(csrf_state)
        .await?
        .ok_or(AuthError::InvalidState)?;

    // Exchange code for claims
    let provider_client = state.get_provider(flow.provider)?;
    let mut claims = provider_client
        .exchange_code(code, &flow.pkce_verifier)
        .await?;

    // Apple: use name from form if available (only sent on first login)
    if flow.provider == OidcProvider::Apple {
        if let Some(name) = apple_name {
            claims.name = Some(name);
        }
    }

    // Find or create user
    let user = find_or_create_user(state, &claims).await?;

    // Create session
    let now = Utc::now();
    let session = Session {
        id: generate_session_id(),
        user_id: user.id.clone(),
        provider: claims.provider,
        created_at: now,
        expires_at: calculate_expiry(now, Duration::seconds(state.config.session_ttl.as_secs() as i64)),
    };
    state.sessions.create_session(&session).await?;

    // Set secure cookie
    let cookie = Cookie::build((&state.config.cookie_name, session.id.to_string()))
        .path("/")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(state.config.session_ttl.as_secs() as i64))
        .build();

    let jar = jar.add(cookie);

    // Redirect to app
    Ok((jar, Redirect::to("/")))
}

async fn find_or_create_user(state: &AuthState, claims: &OidcClaims) -> Result<User, AuthError> {
    // Look up by provider + subject
    if let Some(user) = state
        .users
        .get_user_by_provider(&claims.provider.to_string(), &claims.subject)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?
    {
        return Ok(user);
    }

    // Create new user
    let now = Utc::now();
    let user_id = uuid::Uuid::new_v4().to_string();
    let name = claims
        .name
        .clone()
        .or_else(|| claims.email.as_ref().map(|e| calendsync_core::auth::email_to_name(e)))
        .unwrap_or_else(|| "User".to_string());

    let user = User {
        id: user_id.clone(),
        name: name.clone(),
        email: claims.email.clone(),
        provider: Some(claims.provider.to_string()),
        provider_subject: Some(claims.subject.clone()),
        created_at: now,
        updated_at: now,
    };

    state
        .users
        .create_user(&user)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

    // Create default calendar
    let calendar_id = uuid::Uuid::new_v4().to_string();
    let calendar = Calendar {
        id: calendar_id.clone(),
        name: format!("{}'s Calendar", name),
        is_default: true,
        created_at: now,
        updated_at: now,
    };

    state
        .calendars
        .create_calendar(&calendar)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

    // Create ownership membership
    let membership = CalendarMembership {
        calendar_id,
        user_id: user_id.clone(),
        role: CalendarRole::Owner,
        created_at: now,
    };

    state
        .memberships
        .create_membership(&membership)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

    Ok(user)
}

async fn logout(
    State(state): State<AuthState>,
    CurrentUser(user): CurrentUser,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    // Get session ID from cookie
    if let Some(cookie) = jar.get(&state.config.cookie_name) {
        let session_id = SessionId::new(cookie.value().to_string());
        state.sessions.delete_session(&session_id).await?;
    }

    // Remove cookie
    let jar = jar.remove(Cookie::from(state.config.cookie_name.clone()));
    Ok(jar)
}

async fn logout_all(
    State(state): State<AuthState>,
    CurrentUser(user): CurrentUser,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    state.sessions.delete_user_sessions(&user.id).await?;

    // Remove cookie
    let jar = jar.remove(Cookie::from(state.config.cookie_name.clone()));
    Ok(jar)
}

async fn me(CurrentUser(user): CurrentUser) -> Json<User> {
    Json(user)
}
```

---

### Task 2.9: Implement AuthState

**File:** `crates/auth/src/state.rs`

```rust
use calendsync_core::auth::{OidcProvider, OidcProviderClient, SessionRepository};
use calendsync_core::storage::{CalendarRepository, MembershipRepository, UserRepository};
use std::sync::Arc;

use crate::{AuthConfig, AuthError};
use crate::providers::{AppleProvider, GoogleProvider};

pub struct AuthState {
    pub sessions: Arc<dyn SessionRepository>,
    pub users: Arc<dyn UserRepository>,
    pub calendars: Arc<dyn CalendarRepository>,
    pub memberships: Arc<dyn MembershipRepository>,
    pub config: AuthConfig,
    google: Option<Arc<GoogleProvider>>,
    apple: Option<Arc<AppleProvider>>,
}

impl AuthState {
    pub async fn new(
        sessions: Arc<dyn SessionRepository>,
        users: Arc<dyn UserRepository>,
        calendars: Arc<dyn CalendarRepository>,
        memberships: Arc<dyn MembershipRepository>,
        config: AuthConfig,
    ) -> Result<Self, AuthError> {
        let google = if let Some(ref cfg) = config.google {
            Some(Arc::new(GoogleProvider::new(cfg).await?))
        } else {
            None
        };

        let apple = if let Some(ref cfg) = config.apple {
            Some(Arc::new(AppleProvider::new(cfg).await?))
        } else {
            None
        };

        Ok(Self {
            sessions,
            users,
            calendars,
            memberships,
            config,
            google,
            apple,
        })
    }

    pub fn get_provider(&self, provider: OidcProvider) -> Result<&dyn OidcProviderClient, AuthError> {
        match provider {
            OidcProvider::Google => self
                .google
                .as_ref()
                .map(|p| p.as_ref() as &dyn OidcProviderClient)
                .ok_or_else(|| AuthError::Provider("Google not configured".to_string())),
            OidcProvider::Apple => self
                .apple
                .as_ref()
                .map(|p| p.as_ref() as &dyn OidcProviderClient)
                .ok_or_else(|| AuthError::Provider("Apple not configured".to_string())),
        }
    }
}

impl Clone for AuthState {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            users: self.users.clone(),
            calendars: self.calendars.clone(),
            memberships: self.memberships.clone(),
            config: self.config.clone(),
            google: self.google.clone(),
            apple: self.apple.clone(),
        }
    }
}
```

---

## Phase 3: Mock IdP Server

### Task 3.1: Create mock IdP module

**File:** `crates/auth/src/mock_idp/mod.rs`

```rust
mod server;
mod templates;

pub use server::MockIdpServer;
```

**File:** `crates/auth/src/mock_idp/templates.rs`

```rust
use calendsync_core::auth::OidcProvider;

pub fn login_page(provider: OidcProvider, state: &str, redirect_uri: &str) -> String {
    let provider_name = match provider {
        OidcProvider::Google => "Google",
        OidcProvider::Apple => "Apple",
    };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Mock {provider_name} Sign In (DEV ONLY)</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, sans-serif;
            max-width: 400px;
            margin: 100px auto;
            padding: 20px;
        }}
        .warning {{
            background: #fff3cd;
            border: 1px solid #ffc107;
            padding: 15px;
            border-radius: 8px;
            margin-bottom: 20px;
        }}
        .warning h2 {{
            color: #856404;
            margin-top: 0;
        }}
        form {{
            background: #f8f9fa;
            padding: 20px;
            border-radius: 8px;
        }}
        label {{
            display: block;
            margin-bottom: 5px;
            font-weight: 500;
        }}
        input[type="email"] {{
            width: 100%;
            padding: 10px;
            margin-bottom: 15px;
            border: 1px solid #ced4da;
            border-radius: 4px;
            box-sizing: border-box;
        }}
        button {{
            width: 100%;
            padding: 12px;
            background: #007bff;
            color: white;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 16px;
        }}
        button:hover {{
            background: #0056b3;
        }}
    </style>
</head>
<body>
    <div class="warning">
        <h2>⚠️ Development Only</h2>
        <p>This is a <strong>mock {provider_name} login</strong> for development purposes.</p>
        <p>Enter any email address to simulate authentication.</p>
    </div>

    <form action="/authorize/submit" method="POST">
        <input type="hidden" name="provider" value="{}" />
        <input type="hidden" name="state" value="{state}" />
        <input type="hidden" name="redirect_uri" value="{redirect_uri}" />

        <label for="email">Email Address</label>
        <input type="email" id="email" name="email" placeholder="dev@example.com" required />

        <label for="name">Name (optional)</label>
        <input type="text" id="name" name="name" placeholder="Dev User" />

        <button type="submit">Sign in with {provider_name}</button>
    </form>
</body>
</html>"#,
        provider.to_string()
    )
}
```

**File:** `crates/auth/src/mock_idp/server.rs`

```rust
use axum::{
    extract::{Query, State},
    response::{Html, Redirect},
    routing::{get, post},
    Form, Router,
};
use calendsync_core::auth::OidcProvider;
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use super::templates;

#[derive(Clone)]
struct MockIdpState {
    main_server_url: String,
}

#[derive(Deserialize)]
struct AuthorizeQuery {
    state: String,
    redirect_uri: String,
    #[serde(default)]
    provider: String,
}

#[derive(Deserialize)]
struct LoginForm {
    email: String,
    name: Option<String>,
    state: String,
    redirect_uri: String,
    provider: String,
}

pub struct MockIdpServer {
    port: u16,
    main_server_url: String,
}

impl MockIdpServer {
    pub fn new(port: u16, main_server_url: String) -> Self {
        Self {
            port,
            main_server_url,
        }
    }

    pub async fn run(self) -> Result<(), std::io::Error> {
        let state = MockIdpState {
            main_server_url: self.main_server_url,
        };

        let app = Router::new()
            .route("/google/authorize", get(google_authorize))
            .route("/apple/authorize", get(apple_authorize))
            .route("/authorize/submit", post(authorize_submit))
            .with_state(state);

        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        tracing::info!("Mock IdP server listening on http://{}", addr);

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app).await
    }
}

async fn google_authorize(Query(params): Query<AuthorizeQuery>) -> Html<String> {
    Html(templates::login_page(
        OidcProvider::Google,
        &params.state,
        &params.redirect_uri,
    ))
}

async fn apple_authorize(Query(params): Query<AuthorizeQuery>) -> Html<String> {
    Html(templates::login_page(
        OidcProvider::Apple,
        &params.state,
        &params.redirect_uri,
    ))
}

async fn authorize_submit(
    State(state): State<MockIdpState>,
    Form(form): Form<LoginForm>,
) -> Redirect {
    // Generate a mock authorization code that encodes the user info
    let mock_code = base64::encode(serde_json::json!({
        "email": form.email,
        "name": form.name,
        "provider": form.provider,
        "sub": format!("mock-{}-{}", form.provider, form.email),
    }).to_string());

    let callback_url = format!(
        "{}?code={}&state={}",
        form.redirect_uri,
        urlencoding::encode(&mock_code),
        urlencoding::encode(&form.state),
    );

    Redirect::to(&callback_url)
}
```

Add dependencies to `crates/auth/Cargo.toml`:

```toml
[dependencies]
base64 = { workspace = true }
urlencoding = "2"
```

---

### Task 3.2: Create MockProvider

**File:** `crates/auth/src/providers/mock.rs`

```rust
use async_trait::async_trait;
use calendsync_core::auth::{AuthError, OidcClaims, OidcProvider, OidcProviderClient, Result};
use url::Url;

/// Mock OIDC provider that works with MockIdpServer.
pub struct MockProvider {
    provider: OidcProvider,
    mock_idp_url: Url,
    redirect_uri: Url,
}

impl MockProvider {
    pub fn new(provider: OidcProvider, mock_idp_url: Url, redirect_uri: Url) -> Self {
        Self {
            provider,
            mock_idp_url,
            redirect_uri,
        }
    }
}

#[async_trait]
impl OidcProviderClient for MockProvider {
    async fn authorization_url(&self, state: &str, _pkce_challenge: &str) -> Result<Url> {
        let path = match self.provider {
            OidcProvider::Google => "/google/authorize",
            OidcProvider::Apple => "/apple/authorize",
        };

        let mut url = self.mock_idp_url.join(path)
            .map_err(|e| AuthError::Provider(e.to_string()))?;

        url.query_pairs_mut()
            .append_pair("state", state)
            .append_pair("redirect_uri", self.redirect_uri.as_str());

        Ok(url)
    }

    async fn exchange_code(&self, code: &str, _pkce_verifier: &str) -> Result<OidcClaims> {
        // Decode the mock code (it contains the user info)
        let decoded = base64::decode(code)
            .map_err(|e| AuthError::CodeExchange(e.to_string()))?;

        let json: serde_json::Value = serde_json::from_slice(&decoded)
            .map_err(|e| AuthError::CodeExchange(e.to_string()))?;

        let provider = match json["provider"].as_str() {
            Some("google") => OidcProvider::Google,
            Some("apple") => OidcProvider::Apple,
            _ => return Err(AuthError::CodeExchange("Invalid provider in mock code".to_string())),
        };

        Ok(OidcClaims {
            subject: json["sub"].as_str().unwrap_or("mock-user").to_string(),
            email: json["email"].as_str().map(String::from),
            name: json["name"].as_str().map(String::from),
            provider,
        })
    }

    fn provider(&self) -> OidcProvider {
        self.provider
    }
}
```

---

## Phase 4: Integration with calendsync

### Task 4.1: Update calendsync Cargo.toml

**File:** `crates/calendsync/Cargo.toml`

Add dependency:

```toml
[dependencies]
calendsync_auth = { path = "../auth", features = [] }

[features]
default = ["inmemory", "memory"]
# ... existing features ...
auth-sqlite = ["calendsync_auth/sqlite"]
auth-redis = ["calendsync_auth/redis"]
auth-mock = ["calendsync_auth/mock"]
```

---

### Task 4.2: Update AppState

**File:** `crates/calendsync/src/state.rs`

Add auth state:

```rust
use calendsync_auth::AuthState;

pub struct AppState {
    // Existing
    pub entries: Arc<dyn EntryRepository>,
    pub calendars: Arc<dyn CalendarRepository>,
    // ... existing fields ...

    // New
    pub auth: Option<AuthState>,
}
```

---

### Task 4.3: Wire auth routes in app.rs

**File:** `crates/calendsync/src/app.rs`

Add auth routes:

```rust
use calendsync_auth::auth_routes;

pub fn create_app(state: AppState) -> Router {
    let mut app = Router::new()
        // ... existing routes ...
        ;

    // Add auth routes if auth is configured
    if state.auth.is_some() {
        app = app.merge(auth_routes());
    }

    app.with_state(state)
}
```

---

### Task 4.4: Update main.rs for auth initialization

**File:** `crates/calendsync/src/main.rs`

Add auth setup:

```rust
use calendsync_auth::{AuthConfig, AuthState, SessionStore};

async fn setup_auth(/* pool, config */) -> Option<AuthState> {
    let config = match AuthConfig::from_env() {
        Ok(c) => c,
        Err(_) => {
            tracing::info!("Auth not configured, running without authentication");
            return None;
        }
    };

    #[cfg(feature = "auth-sqlite")]
    let session_store = {
        let store = SessionStore::new(sqlite_pool.clone());
        store.migrate().await.expect("Failed to migrate sessions");
        Arc::new(store)
    };

    #[cfg(feature = "auth-redis")]
    let session_store = Arc::new(SessionStore::new(
        redis_pool.clone(),
        config.session_ttl,
    ));

    let auth_state = AuthState::new(
        session_store,
        user_repo.clone(),
        calendar_repo.clone(),
        membership_repo.clone(),
        config,
    )
    .await
    .expect("Failed to initialize auth");

    Some(auth_state)
}

// In main():
#[cfg(feature = "auth-mock")]
{
    use calendsync_auth::mock_idp::MockIdpServer;

    let mock_server = MockIdpServer::new(3001, "http://localhost:3000".to_string());
    tokio::spawn(async move {
        mock_server.run().await.expect("Mock IdP server failed");
    });
}
```

---

### Task 4.5: Add CurrentUser to protected handlers

**File:** `crates/calendsync/src/handlers/entries.rs`

Update handlers to require authentication:

```rust
use calendsync_auth::CurrentUser;

pub async fn create_entry(
    CurrentUser(user): CurrentUser,  // Add this
    State(state): State<AppState>,
    Json(payload): Json<CreateEntry>,
) -> Result<Json<Entry>, AppError> {
    // Check user has access to calendar
    let membership = state
        .memberships
        .get_membership(&payload.calendar_id, &user.id)
        .await?
        .ok_or(AppError::Forbidden)?;

    if !membership.role.can_write() {
        return Err(AppError::Forbidden);
    }

    // ... rest of handler
}
```

---

## Phase 5: Frontend Updates

### Task 5.1: Add login page component

**File:** `crates/frontend/src/calendsync/components/LoginPage.tsx`

```tsx
import React from "react"

interface LoginPageProps {
  googleEnabled: boolean
  appleEnabled: boolean
}

export function LoginPage({ googleEnabled, appleEnabled }: LoginPageProps) {
  return (
    <div className="login-page">
      <h1>Sign in to Calendsync</h1>

      <div className="login-buttons">
        {googleEnabled && (
          <a href="/auth/google/login" className="login-button google">
            <GoogleIcon />
            Sign in with Google
          </a>
        )}

        {appleEnabled && (
          <a href="/auth/apple/login" className="login-button apple">
            <AppleIcon />
            Sign in with Apple
          </a>
        )}
      </div>
    </div>
  )
}

function GoogleIcon() {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20">
      {/* Google G icon */}
    </svg>
  )
}

function AppleIcon() {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20">
      {/* Apple icon */}
    </svg>
  )
}
```

---

### Task 5.2: Add login styles

**File:** `crates/frontend/src/calendsync/styles/login.css`

```css
.login-page {
  max-width: 400px;
  margin: 100px auto;
  padding: 40px;
  text-align: center;
}

.login-page h1 {
  margin-bottom: 40px;
  font-weight: 600;
}

.login-buttons {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.login-button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 14px 24px;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 500;
  text-decoration: none;
  transition: background-color 0.2s;
}

.login-button.google {
  background: #fff;
  color: #1f1f1f;
  border: 1px solid #dadce0;
}

.login-button.google:hover {
  background: #f8f9fa;
}

.login-button.apple {
  background: #000;
  color: #fff;
  border: 1px solid #000;
}

.login-button.apple:hover {
  background: #1a1a1a;
}
```

---

## Phase 6: Tauri Integration

### Task 6.1: Register deep link URL scheme

**File:** `crates/src-tauri/tauri.conf.json`

Add deep link configuration:

```json
{
  "app": {
    "security": {
      "dangerousRemoteUrlAccess": ["calendsync.app"]
    }
  },
  "plugins": {
    "deep-link": {
      "desktop": {
        "schemes": ["calendsync"]
      },
      "mobile": {
        "schemes": ["calendsync"]
      }
    }
  }
}
```

---

### Task 6.2: Handle deep link auth callback

**File:** `crates/src-tauri/src/auth.rs`

```rust
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

const SESSION_KEY: &str = "session_id";

pub fn handle_deep_link(app: &AppHandle, url: &str) {
    if let Some(captures) = url.strip_prefix("calendsync://auth/callback") {
        // Parse query parameters
        let url = url::Url::parse(&format!("http://dummy{}", captures)).ok();

        if let Some(url) = url {
            let code = url.query_pairs()
                .find(|(k, _)| k == "code")
                .map(|(_, v)| v.to_string());

            let state = url.query_pairs()
                .find(|(k, _)| k == "state")
                .map(|(_, v)| v.to_string());

            if let (Some(code), Some(state)) = (code, state) {
                // Exchange code for session via API
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Ok(session_id) = exchange_code(&code, &state).await {
                        save_session(&app, &session_id);
                    }
                });
            }
        }
    }
}

async fn exchange_code(code: &str, state: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3000/auth/exchange")
        .json(&serde_json::json!({
            "code": code,
            "state": state,
        }))
        .send()
        .await?;

    let json: serde_json::Value = response.json().await?;
    Ok(json["session_id"].as_str().unwrap().to_string())
}

fn save_session(app: &AppHandle, session_id: &str) {
    let store = app.store("auth.json").expect("Failed to get store");
    store.set(SESSION_KEY, serde_json::json!(session_id));
    store.save().expect("Failed to save store");
}

pub fn get_session(app: &AppHandle) -> Option<String> {
    let store = app.store("auth.json").ok()?;
    store.get(SESSION_KEY)?.as_str().map(String::from)
}

pub fn clear_session(app: &AppHandle) {
    if let Ok(store) = app.store("auth.json") {
        store.delete(SESSION_KEY);
        let _ = store.save();
    }
}
```

---

## Phase 7: Documentation

### Task 7.1: Create OIDC setup guide

**File:** `.claude/context/oidc-setup.md`

```markdown
# OIDC Provider Setup Guide

## Google OAuth Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select existing
3. Navigate to APIs & Services → Credentials
4. Click "Create Credentials" → "OAuth client ID"
5. Select "Web application"
6. Add authorized redirect URIs:
   - Production: `https://calendsync.app/auth/google/callback`
   - Development (with ngrok): `https://<your-ngrok>.ngrok.io/auth/google/callback`
7. Copy Client ID and Client Secret

Environment variables:
```bash
GOOGLE_CLIENT_ID=your-client-id
GOOGLE_CLIENT_SECRET=your-client-secret
```

## Apple Sign In Setup

1. Go to [Apple Developer Portal](https://developer.apple.com/)
2. Navigate to Certificates, Identifiers & Profiles
3. Create an App ID with Sign In with Apple capability
4. Create a Services ID for web authentication
5. Configure the Services ID:
   - Domains: `calendsync.app`
   - Return URLs: `https://calendsync.app/auth/apple/callback`
6. Create a Sign In with Apple key (ES256)
7. Download the .p8 key file

Environment variables:
```bash
APPLE_CLIENT_ID=your-services-id
APPLE_TEAM_ID=your-team-id
APPLE_KEY_ID=your-key-id
APPLE_PRIVATE_KEY="-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----"
```

## Development with Mock IdP

For local development without real OIDC providers:

```bash
# Run with mock auth
cargo xtask dev server --auth mock

# Mock IdP runs on port 3001
# Login redirects to http://localhost:3001/google/authorize
# Enter any email to authenticate
```

## Production Checklist

- [ ] Set `COOKIE_SECURE=true`
- [ ] Set `AUTH_BASE_URL=https://calendsync.app`
- [ ] Configure real Google/Apple credentials
- [ ] Use Redis for session storage (`--features auth-redis`)
- [ ] Set appropriate `SESSION_TTL_DAYS` (default: 7)
```

---

### Task 7.2: Create authentication context doc

**File:** `.claude/context/authentication.md`

```markdown
# Authentication Architecture

## Overview

OIDC authentication with Google and Apple, server-side sessions, role-based calendar access.

## Crate Structure

```
calendsync_core (auth feature)
├── Session, OidcClaims types
├── SessionRepository trait
├── OidcProviderClient trait
├── Pure validation functions

calendsync_auth
├── GoogleProvider, AppleProvider
├── SessionStore (sqlite/redis)
├── CurrentUser extractor
├── Auth handlers
├── MockIdpServer (mock feature)

calendsync
├── Wires auth into app
├── Spawns mock IdP in dev
```

## Session Flow

1. User clicks "Sign in with Google/Apple"
2. Server generates PKCE + state, stores temporarily
3. Redirect to provider authorization URL
4. User authenticates at provider
5. Provider redirects to callback with code
6. Server exchanges code for ID token
7. Server creates/finds user, creates session
8. Secure cookie set, user redirected to app

## Authorization

- `CurrentUser` extractor validates session, returns user
- Handlers check `CalendarMembership` for role-based access
- Owner ≥ Writer ≥ Reader permission hierarchy

## Feature Flags

- `auth-sqlite` - SQLite session storage (dev)
- `auth-redis` - Redis session storage (prod)
- `auth-mock` - Mock IdP server (dev only)

## Key Files

- `crates/core/src/auth/` - Pure types and traits
- `crates/auth/src/` - OIDC implementation
- `crates/calendsync/src/handlers/` - Protected handlers
```

---

### Task 7.3: Update CLAUDE.md progressive disclosure table

**File:** `CLAUDE.md`

Add to progressive disclosure table:

```markdown
| Authentication     | `.claude/context/authentication.md` |
| OIDC Setup Guide   | `.claude/context/oidc-setup.md`     |
```

---

## Verification

### Run tests

```bash
# Core auth tests
cargo test -p calendsync_core --features auth

# Auth crate tests
cargo test -p calendsync_auth --features sqlite

# Integration tests
cargo xtask integration --sqlite --features auth-sqlite,auth-mock
```

### Manual E2E test

```bash
# Start with mock auth
cargo xtask dev server --storage sqlite --auth mock --seed

# Visit http://localhost:3000
# Click "Sign in with Google"
# Enter dev@test.com on mock login page
# Verify redirect and session cookie
# Verify /auth/me returns user info
```

---

Plan complete and saved to `.claude/designs/2026-01-03-authentication-design.md`. Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

Which approach?