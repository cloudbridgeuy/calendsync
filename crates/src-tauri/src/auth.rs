//! Authentication module for handling OAuth deep link callbacks.
//!
//! This module provides functionality to:
//! - Handle deep link callbacks from OAuth providers
//! - Exchange authorization codes for session tokens
//! - Persist and retrieve session data using tauri-plugin-store

use tauri::{AppHandle, Emitter};
use tauri_plugin_store::StoreExt;
use tracing::{error, info, warn};
use url::Url;

/// Key used to store the session ID in the auth store.
const SESSION_KEY: &str = "session_id";

/// Key used to store the last-used calendar ID in the auth store.
const LAST_CALENDAR_KEY: &str = "last_calendar_id";

/// Store filename for auth data.
const AUTH_STORE: &str = "auth.json";

/// Handle an incoming deep link URL for authentication.
///
/// Parses the deep link URL and extracts the authorization code and state
/// parameters. If valid, exchanges the code for a session token via the API.
///
/// # Arguments
///
/// * `app` - The Tauri AppHandle
/// * `url` - The deep link URL string (e.g., `calendsync://auth/callback?code=...&state=...`)
pub fn handle_deep_link(app: &AppHandle, url: &str) {
    info!("Received deep link: {}", url);

    // Check if this is an auth callback
    if !url.starts_with("calendsync://auth/callback") {
        info!("Deep link is not an auth callback, ignoring");
        return;
    }

    // Parse the URL to extract query parameters
    // The deep link URL is `calendsync://auth/callback?code=...&state=...`
    // We need to convert it to a parseable format
    let parsed = match Url::parse(url) {
        Ok(u) => u,
        Err(e) => {
            error!("Failed to parse deep link URL: {}", e);
            return;
        }
    };

    // Extract code and state from query parameters
    let code = parsed
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string());

    let state = parsed
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string());

    match (code, state) {
        (Some(code), Some(state)) => {
            info!("Extracted auth callback parameters, emitting to frontend for exchange");
            // Emit auth-code-received event to frontend
            // JavaScript will call /auth/exchange with credentials: include to set the cookie
            if let Err(e) = app.emit(
                "auth-code-received",
                serde_json::json!({ "code": code, "state": state }),
            ) {
                error!("Failed to emit auth-code-received event: {}", e);
            }
        }
        (None, _) => {
            warn!("Auth callback missing 'code' parameter");
        }
        (_, None) => {
            warn!("Auth callback missing 'state' parameter");
        }
    }
}

/// Exchange an authorization code for a session ID.
///
/// Makes an HTTP POST request to the backend `/auth/exchange` endpoint
/// with the authorization code and state.
///
/// Note: This is kept for debugging purposes but is not currently used.
/// The normal flow has JavaScript call /auth/exchange with credentials: include
/// so the cookie is set in the webview.
///
/// # Arguments
///
/// * `code` - The authorization code from the OAuth provider
/// * `state` - The state parameter for CSRF protection
///
/// # Returns
///
/// The session ID on success, or an error.
#[allow(dead_code)]
async fn exchange_code(
    code: &str,
    state: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Make the base URL configurable
    let base_url =
        std::env::var("CALENDSYNC_API_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/auth/exchange", base_url))
        .json(&serde_json::json!({
            "code": code,
            "state": state,
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Auth exchange failed with status {}: {}", status, body).into());
    }

    let json: serde_json::Value = response.json().await?;

    json["session_id"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| "Response missing session_id field".into())
}

/// Save the session ID to persistent storage.
///
/// Uses tauri-plugin-store to persist the session ID to disk.
///
/// # Arguments
///
/// * `app` - The Tauri AppHandle
/// * `session_id` - The session ID to save
pub fn save_session(app: &AppHandle, session_id: &str) {
    match app.store(AUTH_STORE) {
        Ok(store) => {
            store.set(SESSION_KEY, serde_json::json!(session_id));
            if let Err(e) = store.save() {
                error!("Failed to save auth store: {}", e);
            } else {
                info!("Session saved successfully");
            }
        }
        Err(e) => {
            error!("Failed to get auth store: {}", e);
        }
    }
}

/// Retrieve the current session ID from storage.
///
/// # Arguments
///
/// * `app` - The Tauri AppHandle
///
/// # Returns
///
/// The session ID if one exists, or `None`.
pub fn get_session(app: &AppHandle) -> Option<String> {
    let store = app.store(AUTH_STORE).ok()?;
    store.get(SESSION_KEY)?.as_str().map(String::from)
}

/// Clear the current session from storage.
///
/// # Arguments
///
/// * `app` - The Tauri AppHandle
pub fn clear_session(app: &AppHandle) {
    if let Ok(store) = app.store(AUTH_STORE) {
        let _ = store.delete(SESSION_KEY);
        if let Err(e) = store.save() {
            error!("Failed to save auth store after clearing session: {}", e);
        } else {
            info!("Session cleared successfully");
        }
    }
}

/// Retrieve the last-used calendar ID from storage.
///
/// # Arguments
///
/// * `app` - The Tauri AppHandle
///
/// # Returns
///
/// The calendar ID if one exists, or `None`.
pub fn get_last_calendar(app: &AppHandle) -> Option<String> {
    let store = app.store(AUTH_STORE).ok()?;
    store.get(LAST_CALENDAR_KEY)?.as_str().map(String::from)
}

/// Save the last-used calendar ID to persistent storage.
///
/// # Arguments
///
/// * `app` - The Tauri AppHandle
/// * `calendar_id` - The calendar ID to save
pub fn save_last_calendar(app: &AppHandle, calendar_id: &str) {
    match app.store(AUTH_STORE) {
        Ok(store) => {
            store.set(LAST_CALENDAR_KEY, serde_json::json!(calendar_id));
            if let Err(e) = store.save() {
                error!("Failed to save auth store after saving calendar: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to get auth store for saving calendar: {}", e);
        }
    }
}

/// Clear the last-used calendar ID from storage.
///
/// # Arguments
///
/// * `app` - The Tauri AppHandle
pub fn clear_last_calendar(app: &AppHandle) {
    if let Ok(store) = app.store(AUTH_STORE) {
        let _ = store.delete(LAST_CALENDAR_KEY);
        if let Err(e) = store.save() {
            error!("Failed to save auth store after clearing calendar: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_parsing() {
        // Test that we can parse the expected deep link format
        let url = "calendsync://auth/callback?code=abc123&state=xyz789";
        let parsed = Url::parse(url).expect("Should parse successfully");

        let code: Option<String> = parsed
            .query_pairs()
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.to_string());

        let state: Option<String> = parsed
            .query_pairs()
            .find(|(k, _)| k == "state")
            .map(|(_, v)| v.to_string());

        assert_eq!(code, Some("abc123".to_string()));
        assert_eq!(state, Some("xyz789".to_string()));
    }

    #[test]
    fn test_url_parsing_url_encoded() {
        // Test that URL-encoded parameters are decoded correctly
        let url = "calendsync://auth/callback?code=abc%2B123&state=xyz%3D789";
        let parsed = Url::parse(url).expect("Should parse successfully");

        let code: Option<String> = parsed
            .query_pairs()
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.to_string());

        let state: Option<String> = parsed
            .query_pairs()
            .find(|(k, _)| k == "state")
            .map(|(_, v)| v.to_string());

        assert_eq!(code, Some("abc+123".to_string()));
        assert_eq!(state, Some("xyz=789".to_string()));
    }
}
