//! Login page handler.

use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use calendsync_auth::OptionalUser;

use crate::state::AppState;

#[derive(Deserialize, Default)]
pub struct LoginQuery {
    pub return_to: Option<String>,
}

/// Get CSS URL from the manifest.
///
/// In dev mode (DEV_MODE env var set), reads manifest from disk to pick up
/// new hashed filenames after hot-reload. In production, uses compiled-in manifest.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
fn get_css_url() -> String {
    let manifest = get_manifest();

    let css_bundle_name = manifest
        .get("calendsync.css")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync.css");

    format!("/dist/{css_bundle_name}")
}

/// Get the manifest JSON, reading from disk in dev mode or using compiled-in manifest.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
fn get_manifest() -> serde_json::Value {
    // Dev mode: read manifest from disk (picks up new hashed filename after rebuild)
    if std::env::var("DEV_MODE").is_ok() {
        let manifest_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/manifest.json");
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
                return manifest;
            }
        }
        // Fall through to compiled manifest if disk read fails
    }

    // Production: use compiled-in manifest
    let manifest_str = include_str!("../../../frontend/manifest.json");
    serde_json::from_str(manifest_str).unwrap_or(serde_json::json!({}))
}

/// Handler for GET /login
///
/// - Unauthenticated: renders login page
/// - Authenticated: redirects to user's first calendar
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
pub async fn login_page(
    State(state): State<AppState>,
    OptionalUser(user): OptionalUser,
    Query(query): Query<LoginQuery>,
) -> Response {
    // If authenticated, redirect to first calendar
    if let Some(user) = user {
        return redirect_to_first_calendar(&state, user.id).await;
    }

    // Render login page HTML
    render_login_html(&state, query.return_to)
}

/// Redirect an authenticated user to their first calendar.
///
/// If the user has no calendars, redirects back to /login (shouldn't happen but handle gracefully).
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
async fn redirect_to_first_calendar(state: &AppState, user_id: uuid::Uuid) -> Response {
    let auth = match &state.auth {
        Some(auth) => auth,
        None => {
            tracing::error!("Auth state not initialized");
            return Redirect::to("/login").into_response();
        }
    };

    match auth.memberships.get_calendars_for_user(user_id).await {
        Ok(calendars) if !calendars.is_empty() => {
            let first_calendar_id = calendars[0].0.id;
            Redirect::to(&format!("/calendar/{first_calendar_id}")).into_response()
        }
        Ok(_) => {
            // No calendars found - redirect back to login
            tracing::warn!(user_id = %user_id, "User has no calendars");
            Redirect::to("/login").into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get user calendars");
            Redirect::to("/login").into_response()
        }
    }
}

/// Render the login page HTML.
///
/// Builds a simple HTML page with login buttons for each enabled provider.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
fn render_login_html(state: &AppState, return_to: Option<String>) -> Response {
    let auth = match &state.auth {
        Some(auth) => auth,
        None => {
            tracing::error!("Auth state not initialized");
            return Html("<h1>Authentication not configured</h1>").into_response();
        }
    };

    // Get CSS URL from manifest
    let css_url = get_css_url();

    // Build return_to query parameter for auth links
    let return_to_param = return_to
        .as_ref()
        .map(|r| format!("?return_to={}", urlencoding::encode(r)))
        .unwrap_or_default();

    // Build login buttons for enabled providers
    let mut buttons = String::new();

    if auth.config.google.is_some() {
        // Google logo SVG with brand colors
        let google_svg = r##"<svg class="login-button-icon" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" fill="#4285F4"/>
                    <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853"/>
                    <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" fill="#FBBC05"/>
                    <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335"/>
                </svg>"##;
        buttons.push_str(&format!(
            r#"<a href="/auth/google/login{return_to_param}" class="login-button login-button-google">
                {google_svg}
                Continue with Google
            </a>"#
        ));
    }

    if auth.config.apple.is_some() {
        // Apple logo SVG
        let apple_svg = r##"<svg class="login-button-icon" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path d="M17.05 20.28c-.98.95-2.05.8-3.08.35-1.09-.46-2.09-.48-3.24 0-1.44.62-2.2.44-3.06-.35C2.79 15.25 3.51 7.59 9.05 7.31c1.35.07 2.29.74 3.08.8 1.18-.24 2.31-.93 3.57-.84 1.51.12 2.65.72 3.4 1.8-3.12 1.87-2.38 5.98.48 7.13-.57 1.5-1.31 2.99-2.53 4.08zM12.03 7.25c-.15-2.23 1.66-4.07 3.74-4.25.29 2.58-2.34 4.5-3.74 4.25z" fill="currentColor"/>
                </svg>"##;
        buttons.push_str(&format!(
            r#"<a href="/auth/apple/login{return_to_param}" class="login-button login-button-apple">
                {apple_svg}
                Continue with Apple
            </a>"#
        ));
    }

    // If no providers are configured, show a message
    if buttons.is_empty() {
        buttons = r#"<p class="login-no-providers">No authentication providers configured.</p>"#
            .to_string();
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Login - CalendSync</title>
    <link rel="stylesheet" href="{css_url}">
    <style>
        .login-page {{
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            padding: 2rem;
            background: var(--bg-primary, #f5f5f5);
        }}
        .login-container {{
            background: var(--bg-secondary, #fff);
            border-radius: 12px;
            padding: 2.5rem;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
            max-width: 400px;
            width: 100%;
            text-align: center;
        }}
        .login-title {{
            margin: 0 0 0.5rem 0;
            font-size: 1.75rem;
            color: var(--text-primary, #333);
        }}
        .login-subtitle {{
            margin: 0 0 2rem 0;
            color: var(--text-secondary, #666);
            font-size: 0.95rem;
        }}
        .login-buttons {{
            display: flex;
            flex-direction: column;
            gap: 1rem;
        }}
        .login-button {{
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 0.75rem;
            padding: 0.875rem 1.5rem;
            border-radius: 8px;
            font-size: 1rem;
            font-weight: 500;
            text-decoration: none;
            transition: background-color 0.2s, transform 0.1s;
            cursor: pointer;
        }}
        .login-button:hover {{
            transform: translateY(-1px);
        }}
        .login-button:active {{
            transform: translateY(0);
        }}
        .login-button-icon {{
            width: 20px;
            height: 20px;
        }}
        .login-button-google {{
            background: #fff;
            color: #333;
            border: 1px solid #ddd;
        }}
        .login-button-google:hover {{
            background: #f8f8f8;
        }}
        .login-button-apple {{
            background: #000;
            color: #fff;
            border: 1px solid #000;
        }}
        .login-button-apple:hover {{
            background: #333;
        }}
        .login-no-providers {{
            color: var(--text-secondary, #666);
            font-style: italic;
        }}
    </style>
</head>
<body>
    <div class="login-page">
        <div class="login-container">
            <h1 class="login-title">Welcome to CalendSync</h1>
            <p class="login-subtitle">Sign in to access your calendars</p>
            <div class="login-buttons">
                {buttons}
            </div>
        </div>
    </div>
</body>
</html>"#
    );

    Html(html).into_response()
}
