//! Google OIDC provider implementation.

use async_trait::async_trait;
use calendsync_core::auth::{
    generate_state, AuthError, OidcClaims, OidcProvider, OidcProviderClient, Result,
};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointSet,
    IssuerUrl, Nonce, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use url::Url;

use crate::config::ProviderConfig;

/// Type alias for a CoreClient configured from provider metadata.
///
/// `from_provider_metadata` returns a client with:
/// - HasAuthUrl = EndpointSet (always set from discovery)
/// - HasDeviceAuthUrl = EndpointNotSet
/// - HasIntrospectionUrl = EndpointNotSet
/// - HasRevocationUrl = EndpointNotSet
/// - HasTokenUrl = EndpointMaybeSet (may or may not be in discovery)
/// - HasUserInfoUrl = EndpointMaybeSet (may or may not be in discovery)
///
/// Calling `set_redirect_uri` preserves these type parameters.
type ConfiguredCoreClient = CoreClient<
    EndpointSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

/// Google OIDC provider.
pub struct GoogleProvider {
    client: ConfiguredCoreClient,
    http_client: reqwest::Client,
}

impl GoogleProvider {
    /// Create a new Google provider by discovering the OIDC metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The issuer URL is invalid
    /// - Discovery fails (network error or invalid metadata)
    /// - The redirect URI is invalid
    pub async fn new(config: &ProviderConfig) -> Result<Self> {
        let issuer_url = IssuerUrl::new("https://accounts.google.com".to_string())
            .map_err(|e| AuthError::Provider(e.to_string()))?;

        // Build HTTP client without redirect following (security requirement)
        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| AuthError::Provider(format!("Failed to build HTTP client: {}", e)))?;

        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
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

        Ok(Self {
            client,
            http_client,
        })
    }
}

#[async_trait]
impl OidcProviderClient for GoogleProvider {
    async fn authorization_url(&self, state: &str, pkce_challenge: &str) -> Result<Url> {
        // Clone state and pkce_challenge to avoid lifetime issues with closures
        let state_owned = state.to_string();
        let pkce_challenge_owned = pkce_challenge.to_string();

        // Use add_extra_param to pass the pre-computed PKCE challenge directly.
        // The pkce_challenge parameter is the base64url-encoded SHA256 hash of the verifier.
        let (auth_url, _csrf_token, _nonce) = self
            .client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                move || CsrfToken::new(state_owned),
                || Nonce::new(generate_state()),
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .add_extra_param("code_challenge", pkce_challenge_owned)
            .add_extra_param("code_challenge_method", "S256")
            .url();

        Ok(auth_url)
    }

    async fn exchange_code(&self, code: &str, pkce_verifier: &str) -> Result<OidcClaims> {
        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|e| AuthError::CodeExchange(e.to_string()))?
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.to_string()))
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
            provider: OidcProvider::Google,
        })
    }

    fn provider(&self) -> OidcProvider {
        OidcProvider::Google
    }
}
