//! Mock IdP server for development and testing.
//!
//! This server simulates Google and Apple OIDC authorization endpoints,
//! allowing full E2E testing of the auth flow without real providers.

use axum::{
    extract::Query,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use base64::Engine;
use calendsync_core::auth::OidcProvider;
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use super::templates;

#[derive(Clone)]
struct MockIdpState;

#[derive(Deserialize)]
struct AuthorizeQuery {
    state: String,
    redirect_uri: String,
}

#[derive(Deserialize)]
struct LoginForm {
    email: String,
    name: Option<String>,
    state: String,
    redirect_uri: String,
    provider: String,
}

/// Mock IdP server that simulates OIDC authorization endpoints.
pub struct MockIdpServer {
    port: u16,
}

impl MockIdpServer {
    /// Create a new Mock IdP server.
    ///
    /// # Arguments
    /// * `port` - The port to listen on (typically 3001)
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    /// Run the Mock IdP server.
    ///
    /// This starts an HTTP server that handles:
    /// - `GET /google/authorize` - Google login page
    /// - `GET /apple/authorize` - Apple login page
    /// - `POST /authorize/submit` - Form submission handler
    pub async fn run(self) -> Result<(), std::io::Error> {
        let state = MockIdpState;

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

async fn authorize_submit(Form(form): Form<LoginForm>) -> Response {
    // Generate a mock authorization code that encodes the user info
    let mock_code = base64::engine::general_purpose::STANDARD.encode(
        serde_json::json!({
            "email": form.email,
            "name": form.name,
            "provider": form.provider,
            "sub": format!("mock-{}-{}", form.provider, form.email),
        })
        .to_string(),
    );

    let encoded_code = urlencoding::encode(&mock_code);
    let encoded_state = urlencoding::encode(&form.state);

    // Apple uses form_post response mode - return auto-submitting form
    if form.provider == "apple" {
        let html = templates::form_post_page(&form.redirect_uri, &encoded_code, &encoded_state);
        return Html(html).into_response();
    }

    // Google uses standard redirect with query params
    let callback_url = format!(
        "{}?code={}&state={}",
        form.redirect_uri, encoded_code, encoded_state,
    );
    Redirect::to(&callback_url).into_response()
}
