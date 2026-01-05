//! Mock OIDC provider for development and testing.
//!
//! This module provides a mock implementation of `OidcProviderClient`
//! that works with the Mock IdP server for local development.

use async_trait::async_trait;
use base64::Engine;
use calendsync_core::auth::{AuthError, OidcClaims, OidcProvider, OidcProviderClient, Result};
use url::Url;

/// Mock OIDC provider that works with MockIdpServer.
///
/// This provider generates authorization URLs that point to the Mock IdP server
/// and can decode mock authorization codes that contain embedded user info.
pub struct MockProvider {
    provider: OidcProvider,
    mock_idp_url: Url,
    redirect_uri: Url,
}

impl MockProvider {
    /// Create a new MockProvider.
    ///
    /// # Arguments
    /// * `provider` - The OIDC provider to simulate (Google or Apple)
    /// * `mock_idp_url` - The URL of the Mock IdP server (e.g., http://localhost:3001)
    /// * `redirect_uri` - The callback URL for the main app
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

        let mut url = self
            .mock_idp_url
            .join(path)
            .map_err(|e| AuthError::Provider(e.to_string()))?;

        url.query_pairs_mut()
            .append_pair("state", state)
            .append_pair("redirect_uri", self.redirect_uri.as_str());

        Ok(url)
    }

    async fn exchange_code(&self, code: &str, _pkce_verifier: &str) -> Result<OidcClaims> {
        // Decode the mock code (it contains the user info)
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(code)
            .map_err(|e| AuthError::CodeExchange(e.to_string()))?;

        let json: serde_json::Value =
            serde_json::from_slice(&decoded).map_err(|e| AuthError::CodeExchange(e.to_string()))?;

        let provider = match json["provider"].as_str() {
            Some("google") => OidcProvider::Google,
            Some("apple") => OidcProvider::Apple,
            _ => {
                return Err(AuthError::CodeExchange(
                    "Invalid provider in mock code".to_string(),
                ))
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authorization_url_google() {
        let provider = MockProvider::new(
            OidcProvider::Google,
            Url::parse("http://localhost:3001").unwrap(),
            Url::parse("http://localhost:3000/auth/callback").unwrap(),
        );

        let url = provider
            .authorization_url("test-state", "test-challenge")
            .await
            .unwrap();

        assert!(url.path().contains("/google/authorize"));
        assert!(url.query().unwrap().contains("state=test-state"));
    }

    #[tokio::test]
    async fn test_authorization_url_apple() {
        let provider = MockProvider::new(
            OidcProvider::Apple,
            Url::parse("http://localhost:3001").unwrap(),
            Url::parse("http://localhost:3000/auth/callback").unwrap(),
        );

        let url = provider
            .authorization_url("test-state", "test-challenge")
            .await
            .unwrap();

        assert!(url.path().contains("/apple/authorize"));
    }

    #[tokio::test]
    async fn test_exchange_code() {
        let provider = MockProvider::new(
            OidcProvider::Google,
            Url::parse("http://localhost:3001").unwrap(),
            Url::parse("http://localhost:3000/auth/callback").unwrap(),
        );

        // Create a mock code like the Mock IdP server would
        let mock_code = base64::engine::general_purpose::STANDARD.encode(
            serde_json::json!({
                "email": "test@example.com",
                "name": "Test User",
                "provider": "google",
                "sub": "mock-google-test@example.com",
            })
            .to_string(),
        );

        let claims = provider
            .exchange_code(&mock_code, "verifier")
            .await
            .unwrap();

        assert_eq!(claims.email, Some("test@example.com".to_string()));
        assert_eq!(claims.name, Some("Test User".to_string()));
        assert_eq!(claims.subject, "mock-google-test@example.com");
        assert_eq!(claims.provider, OidcProvider::Google);
    }

    #[tokio::test]
    async fn test_exchange_code_invalid() {
        let provider = MockProvider::new(
            OidcProvider::Google,
            Url::parse("http://localhost:3001").unwrap(),
            Url::parse("http://localhost:3000/auth/callback").unwrap(),
        );

        let result = provider.exchange_code("invalid-code", "verifier").await;
        assert!(result.is_err());
    }
}
