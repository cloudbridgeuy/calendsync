//! Tauri IPC commands for frontend communication.
//!
//! This module provides commands that the frontend can invoke via Tauri's IPC system:
//! - Session management (get/clear)
//! - Last calendar ID storage (get/set)
//! - OAuth login initiation
//! - HTTP proxy commands for server communication

use tauri::AppHandle;

use crate::http::{self, CalendarWithRole, CreateEntryPayload, ServerDay, ServerEntry};

/// Get the current session ID from persistent storage.
///
/// # Returns
///
/// The session ID if one exists, or `None`.
#[tauri::command]
pub fn get_session(app: AppHandle) -> Option<String> {
    crate::auth::get_session(&app)
}

/// Save a session ID to persistent storage.
///
/// # Arguments
///
/// * `session_id` - The session ID to save
#[tauri::command]
pub fn set_session(app: AppHandle, session_id: String) {
    crate::auth::save_session(&app, &session_id)
}

/// Clear the current session from persistent storage.
#[tauri::command]
pub fn clear_session(app: AppHandle) {
    crate::auth::clear_session(&app)
}

/// Get the last-used calendar ID from persistent storage.
///
/// # Returns
///
/// The calendar ID if one exists, or `None`.
#[tauri::command]
pub fn get_last_calendar(app: AppHandle) -> Option<String> {
    crate::auth::get_last_calendar(&app)
}

/// Save the last-used calendar ID to persistent storage.
///
/// # Arguments
///
/// * `calendar_id` - The calendar ID to save
#[tauri::command]
pub fn set_last_calendar(app: AppHandle, calendar_id: String) {
    crate::auth::save_last_calendar(&app, &calendar_id)
}

/// Clear the last-used calendar ID from persistent storage.
#[tauri::command]
pub fn clear_last_calendar(app: AppHandle) {
    crate::auth::clear_last_calendar(&app)
}

/// The redirect URI for OAuth callbacks in the Tauri app.
const AUTH_CALLBACK_URI: &str = "calendsync://auth/callback";

/// Open the system browser to initiate OAuth login with the specified provider.
///
/// # Arguments
///
/// * `provider` - The OAuth provider name (e.g., "google", "apple")
///
/// # Returns
///
/// `Ok(())` on success, or an error message if the browser could not be opened
/// or if the provider is not recognized.
#[tauri::command]
pub fn open_oauth_login(provider: String) -> Result<(), String> {
    // Validate provider to prevent path traversal
    if provider != "google" && provider != "apple" {
        return Err(format!("Unknown OAuth provider: {}", provider));
    }

    let base_url =
        std::env::var("CALENDSYNC_API_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let url = format!(
        "{}/auth/{}/login?redirect_uri={}",
        base_url,
        provider,
        urlencoding::encode(AUTH_CALLBACK_URI)
    );
    open::that(&url).map_err(|e| e.to_string())
}

// HTTP proxy commands - route frontend requests through Rust to bypass CSP/CORS

/// Exchange authorization code for session (calls server /auth/exchange).
#[tauri::command]
pub async fn exchange_auth_code(
    app: AppHandle,
    code: String,
    state: String,
) -> Result<String, String> {
    http::exchange_auth_code(&app, &code, &state).await
}

/// Validate current session (calls server /auth/me).
#[tauri::command]
pub async fn validate_session(app: AppHandle) -> Result<bool, String> {
    http::validate_session(&app).await
}

/// Log out the current session (calls server /auth/logout).
#[tauri::command]
pub async fn logout(app: AppHandle) -> Result<(), String> {
    http::logout(&app).await
}

/// Fetch user's calendars (calls server /api/calendars/me).
#[tauri::command]
pub async fn fetch_my_calendars(app: AppHandle) -> Result<Vec<CalendarWithRole>, String> {
    http::fetch_my_calendars(&app).await
}

/// Fetch calendar entries for a date range (calls server /api/entries).
#[tauri::command]
pub async fn fetch_entries(
    app: AppHandle,
    calendar_id: String,
    highlighted_day: String,
    before: Option<i32>,
    after: Option<i32>,
) -> Result<Vec<ServerDay>, String> {
    http::fetch_entries(&app, &calendar_id, &highlighted_day, before, after).await
}

/// Create a new entry (calls server POST /api/entries).
#[tauri::command]
pub async fn create_entry(
    app: AppHandle,
    payload: CreateEntryPayload,
) -> Result<ServerEntry, String> {
    http::create_entry(&app, payload).await
}

/// Update an existing entry (calls server PUT /api/entries/{id}).
#[tauri::command]
pub async fn update_entry(
    app: AppHandle,
    id: String,
    payload: CreateEntryPayload,
) -> Result<ServerEntry, String> {
    http::update_entry(&app, &id, payload).await
}

/// Delete an entry (calls server DELETE /api/entries/{id}).
#[tauri::command]
pub async fn delete_entry(app: AppHandle, id: String) -> Result<(), String> {
    http::delete_entry(&app, &id).await
}

/// Toggle a task's completed status (calls server PATCH /api/entries/{id}/toggle).
#[tauri::command]
pub async fn toggle_entry(app: AppHandle, id: String) -> Result<ServerEntry, String> {
    http::toggle_entry(&app, &id).await
}
