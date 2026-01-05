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
    /// Returns an error if a provider is partially configured (e.g., client ID without secret).
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
