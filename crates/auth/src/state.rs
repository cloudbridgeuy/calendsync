//! Application state for auth.

use axum::extract::FromRef;
use calendsync_core::auth::{OidcProvider, OidcProviderClient, SessionRepository};
use calendsync_core::storage::{CalendarRepository, MembershipRepository, UserRepository};
use std::sync::Arc;

use crate::config::AuthConfig;
use crate::error::AuthError;

#[cfg(not(feature = "mock"))]
use crate::providers::{AppleProvider, GoogleProvider};

#[cfg(feature = "mock")]
use crate::providers::MockProvider;

/// Shared state for auth handlers.
pub struct AuthState {
    pub sessions: Arc<dyn SessionRepository>,
    pub users: Arc<dyn UserRepository>,
    pub calendars: Arc<dyn CalendarRepository>,
    pub memberships: Arc<dyn MembershipRepository>,
    pub config: AuthConfig,
    #[cfg(not(feature = "mock"))]
    google: Option<Arc<GoogleProvider>>,
    #[cfg(not(feature = "mock"))]
    apple: Option<Arc<AppleProvider>>,
    #[cfg(feature = "mock")]
    google: Option<Arc<MockProvider>>,
    #[cfg(feature = "mock")]
    apple: Option<Arc<MockProvider>>,
}

impl AuthState {
    /// Creates a new AuthState with all required repositories and providers.
    ///
    /// # Errors
    ///
    /// Returns an error if provider initialization fails (e.g., OIDC discovery).
    #[cfg(not(feature = "mock"))]
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

    /// Creates a new AuthState with mock providers for development.
    #[cfg(feature = "mock")]
    pub async fn new(
        sessions: Arc<dyn SessionRepository>,
        users: Arc<dyn UserRepository>,
        calendars: Arc<dyn CalendarRepository>,
        memberships: Arc<dyn MembershipRepository>,
        config: AuthConfig,
    ) -> Result<Self, AuthError> {
        use calendsync_core::auth::OidcProvider;
        use url::Url;

        let mock_idp_url =
            Url::parse("http://localhost:3001").map_err(|e| AuthError::Config(e.to_string()))?;

        let google = if config.google.is_some() {
            Some(Arc::new(MockProvider::new(
                OidcProvider::Google,
                mock_idp_url.clone(),
                config.base_url.join("/auth/google/callback").unwrap(),
            )))
        } else {
            None
        };

        let apple = if config.apple.is_some() {
            Some(Arc::new(MockProvider::new(
                OidcProvider::Apple,
                mock_idp_url,
                config.base_url.join("/auth/apple/callback").unwrap(),
            )))
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

    /// Gets the provider client for the given OIDC provider.
    ///
    /// # Errors
    ///
    /// Returns `ProviderNotConfigured` if the provider is not enabled.
    pub fn get_provider(
        &self,
        provider: OidcProvider,
    ) -> Result<&dyn OidcProviderClient, AuthError> {
        match provider {
            OidcProvider::Google => self
                .google
                .as_ref()
                .map(|p| p.as_ref() as &dyn OidcProviderClient)
                .ok_or_else(|| AuthError::ProviderNotConfigured("Google".to_string())),
            OidcProvider::Apple => self
                .apple
                .as_ref()
                .map(|p| p.as_ref() as &dyn OidcProviderClient)
                .ok_or_else(|| AuthError::ProviderNotConfigured("Apple".to_string())),
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

/// Allows AuthState to be extracted from a parent state.
impl<S> FromRef<S> for AuthState
where
    S: AsRef<AuthState>,
{
    fn from_ref(state: &S) -> Self {
        state.as_ref().clone()
    }
}
