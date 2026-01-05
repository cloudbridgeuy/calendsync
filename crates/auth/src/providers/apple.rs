//! Apple OIDC provider implementation.
//!
//! Apple Sign In requires a signed JWT as the client secret, generated using
//! the team's ES256 private key. This is different from Google which uses
//! a static client secret string.

use async_trait::async_trait;
use calendsync_core::auth::{
    generate_state, AuthError, OidcClaims, OidcProvider, OidcProviderClient, Result,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest, AuthorizationCode, ClientId, CsrfToken, EndpointMaybeSet, EndpointSet, IssuerUrl,
    Nonce, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

use crate::config::AppleConfig;

/// Type alias for a CoreClient configured from provider metadata.
///
/// See google.rs for detailed documentation on these type parameters.
type ConfiguredCoreClient = CoreClient<
    EndpointSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

/// Apple OIDC provider.
pub struct AppleProvider {
    client: ConfiguredCoreClient,
    http_client: reqwest::Client,
    config: AppleConfig,
}

impl AppleProvider {
    /// Create a new Apple provider by discovering the OIDC metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The issuer URL is invalid
    /// - Discovery fails (network error or invalid metadata)
    /// - The redirect URI is invalid
    pub async fn new(config: &AppleConfig) -> Result<Self> {
        let issuer_url = IssuerUrl::new("https://appleid.apple.com".to_string())
            .map_err(|e| AuthError::Provider(e.to_string()))?;

        // Build HTTP client without redirect following (security requirement)
        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| AuthError::Provider(format!("Failed to build HTTP client: {}", e)))?;

        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
            .await
            .map_err(|e| AuthError::Provider(e.to_string()))?;

        // Apple uses signed JWT as client secret, so we don't set one here.
        // The client secret is generated per-request in exchange_code.
        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(config.client_id.clone()),
            None,
        )
        .set_redirect_uri(
            RedirectUrl::new(config.redirect_uri.to_string())
                .map_err(|e| AuthError::Provider(e.to_string()))?,
        );

        Ok(Self {
            client,
            http_client,
            config: config.clone(),
        })
    }

    /// Generate Apple client secret (signed JWT).
    ///
    /// Apple requires a signed JWT as the client_secret for token requests.
    /// See: <https://developer.apple.com/documentation/sign_in_with_apple/generate_and_validate_tokens>
    fn generate_client_secret(&self) -> Result<String> {
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
            .map_err(|_| AuthError::Provider("System clock before UNIX epoch".to_string()))?
            .as_secs();

        let claims = Claims {
            iss: self.config.team_id.clone(),
            iat: now,
            exp: now + 86400 * 180, // 180 days max per Apple docs
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
        // Clone state and pkce_challenge to avoid lifetime issues with closures
        let state_owned = state.to_string();
        let pkce_challenge_owned = pkce_challenge.to_string();

        // Use add_extra_param to pass the pre-computed PKCE challenge directly.
        // Apple also requires response_mode=form_post for security.
        let (auth_url, _csrf_token, _nonce) = self
            .client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                move || CsrfToken::new(state_owned),
                || Nonce::new(generate_state()),
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("name".to_string()))
            .add_extra_param("code_challenge", pkce_challenge_owned)
            .add_extra_param("code_challenge_method", "S256")
            .add_extra_param("response_mode", "form_post") // Apple requires this
            .url();

        Ok(auth_url)
    }

    async fn exchange_code(&self, code: &str, pkce_verifier: &str) -> Result<OidcClaims> {
        // Generate fresh client secret JWT for this request
        let client_secret = self.generate_client_secret()?;

        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|e| AuthError::CodeExchange(e.to_string()))?
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.to_string()))
            .add_extra_param("client_secret", &client_secret)
            .request_async(&self.http_client)
            .await
            .map_err(|e| AuthError::CodeExchange(e.to_string()))?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| AuthError::InvalidToken("No ID token in response".to_string()))?;

        let claims = id_token
            .claims(&self.client.id_token_verifier(), |_: Option<&Nonce>| Ok(()))
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
