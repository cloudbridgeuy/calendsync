use std::time::Duration;

use thiserror::Error;
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

/// Errors that can occur while loading auth configuration from the environment.
#[derive(Debug, Error)]
pub enum AuthConfigError {
    /// A required environment variable was missing or invalid, e.g. a provider's
    /// client ID was set without its matching secret.
    #[error("invalid auth environment variable: {0}")]
    EnvVar(#[from] std::env::VarError),

    /// Neither Google nor Apple was configured.
    #[error(
        "no auth provider configured: set GOOGLE_CLIENT_ID or APPLE_CLIENT_ID \
         (with their required credentials)"
    )]
    NoProviderConfigured,
}

/// A provider found configured while reading the environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfiguredProvider {
    Google,
    Apple,
}

/// Decides whether a configuration is valid, given the providers it configured.
///
/// This is the pure decision behind rejecting a zero-provider configuration,
/// kept separate from `from_env`'s environment reads so it can be unit tested
/// without touching the environment.
fn require_at_least_one_provider(providers: &[ConfiguredProvider]) -> Result<(), AuthConfigError> {
    if providers.is_empty() {
        Err(AuthConfigError::NoProviderConfigured)
    } else {
        Ok(())
    }
}

impl AuthConfig {
    /// Load from environment variables.
    ///
    /// # Environment Variables
    ///
    /// - `AUTH_BASE_URL`: Base URL for callback redirects (default: `http://localhost:3000`)
    /// - `GOOGLE_CLIENT_ID`: Google OAuth client ID (optional, enables Google auth)
    /// - `GOOGLE_CLIENT_SECRET`: Google OAuth client secret (required if Google enabled)
    /// - `APPLE_CLIENT_ID`: Apple OAuth client ID (optional, enables Apple auth)
    /// - `APPLE_TEAM_ID`: Apple developer team ID (required if Apple enabled)
    /// - `APPLE_KEY_ID`: Apple key ID (required if Apple enabled)
    /// - `APPLE_PRIVATE_KEY`: Apple ES256 private key PEM (required if Apple enabled)
    /// - `SESSION_TTL_DAYS`: Session TTL in days (default: 7)
    /// - `COOKIE_SECURE`: Whether to set secure flag on cookies (default: true)
    ///
    /// # Errors
    ///
    /// Returns an error if a provider is partially configured (e.g., client ID
    /// without secret), or if no provider is configured at all.
    pub fn from_env() -> Result<Self, AuthConfigError> {
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

        let mut configured = Vec::new();
        if google.is_some() {
            configured.push(ConfiguredProvider::Google);
        }
        if apple.is_some() {
            configured.push(ConfiguredProvider::Apple);
        }
        require_at_least_one_provider(&configured)?;

        let session_ttl = std::env::var("SESSION_TTL_DAYS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(|days| Duration::from_secs(days * 24 * 60 * 60))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn require_at_least_one_provider_rejects_empty() {
        let result = require_at_least_one_provider(&[]);
        assert!(matches!(result, Err(AuthConfigError::NoProviderConfigured)));
    }

    #[test]
    fn require_at_least_one_provider_accepts_google() {
        let result = require_at_least_one_provider(&[ConfiguredProvider::Google]);
        assert!(result.is_ok());
    }

    #[test]
    fn require_at_least_one_provider_accepts_apple() {
        let result = require_at_least_one_provider(&[ConfiguredProvider::Apple]);
        assert!(result.is_ok());
    }

    // `from_env` reads process-wide environment variables, so these tests serialize
    // on a lock and restore whatever was there before, to avoid interfering with
    // each other or with anything else that reads these variables.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    const AUTH_ENV_VARS: &[&str] = &[
        "GOOGLE_CLIENT_ID",
        "GOOGLE_CLIENT_SECRET",
        "APPLE_CLIENT_ID",
        "APPLE_TEAM_ID",
        "APPLE_KEY_ID",
        "APPLE_PRIVATE_KEY",
    ];

    struct EnvGuard {
        saved: Vec<(&'static str, Option<String>)>,
    }

    impl EnvGuard {
        fn clear_all() -> Self {
            let saved = AUTH_ENV_VARS
                .iter()
                .map(|&key| (key, std::env::var(key).ok()))
                .collect();
            for &key in AUTH_ENV_VARS {
                std::env::remove_var(key);
            }
            Self { saved }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in &self.saved {
                match value {
                    Some(v) => std::env::set_var(key, v),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    #[test]
    fn from_env_rejects_zero_providers() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::clear_all();

        let result = AuthConfig::from_env();

        assert!(matches!(result, Err(AuthConfigError::NoProviderConfigured)));
    }

    #[test]
    fn from_env_rejects_partial_google_config() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::clear_all();
        std::env::set_var("GOOGLE_CLIENT_ID", "test-client-id");
        // GOOGLE_CLIENT_SECRET intentionally left unset.

        let result = AuthConfig::from_env();

        assert!(matches!(result, Err(AuthConfigError::EnvVar(_))));
    }
}
