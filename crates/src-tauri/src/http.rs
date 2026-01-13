//! HTTP client for server communication.
//!
//! This module provides HTTP functions that the Tauri commands use to communicate
//! with the calendsync server. The Rust HTTP client bypasses browser CSP/CORS restrictions.

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::auth::{get_session, save_session};

/// Format HTTP error message with operation, status code, and response body.
///
/// This is a pure function (Functional Core) for consistent error formatting.
///
/// # Examples
///
/// ```no_run
/// # use reqwest::StatusCode;
/// # fn format_http_error(operation: &str, status: StatusCode, body: &str) -> String {
/// #     format!("Failed to {}: {} - {}", operation, status, body)
/// # }
/// let error = format_http_error("create entry", StatusCode::BAD_REQUEST, "Invalid date format");
/// assert_eq!(error, "Failed to create entry: 400 Bad Request - Invalid date format");
/// ```
fn format_http_error(operation: &str, status: reqwest::StatusCode, body: &str) -> String {
    format!("Failed to {}: {} - {}", operation, status, body)
}

/// Get API base URL (build-time constant).
pub fn api_url() -> &'static str {
    #[cfg(debug_assertions)]
    {
        "http://localhost:3000"
    }
    #[cfg(not(debug_assertions))]
    {
        "https://api.calendsync.app"
    }
}

/// Calendar with user's role/permission level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarWithRole {
    pub id: String,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub role: String,
}

/// Calendar entry from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEntry {
    pub id: String,
    pub calendar_id: String,
    pub kind: String,
    pub completed: bool,
    pub is_multi_day: bool,
    pub is_all_day: bool,
    pub is_timed: bool,
    pub is_task: bool,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub color: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

/// A day with its entries from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDay {
    pub date: String,
    pub entries: Vec<ServerEntry>,
}

/// Payload for creating or updating an entry.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateEntryPayload {
    pub calendar_id: String,
    pub title: String,
    pub date: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub all_day: Option<bool>,
    pub description: Option<String>,
    pub entry_type: Option<String>,
}

/// Get a configured HTTP client with the session cookie.
fn client_with_session(app: &AppHandle) -> Result<(reqwest::Client, String), String> {
    let session_id = get_session(app).ok_or("No session")?;
    let client = reqwest::Client::new();
    Ok((client, session_id))
}

/// Exchange authorization code for session.
pub async fn exchange_auth_code(
    app: &AppHandle,
    code: &str,
    state: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/auth/exchange", api_url()))
        .json(&serde_json::json!({ "code": code, "state": state }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format_http_error("exchange auth code", status, &body));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let session_id = json["session_id"]
        .as_str()
        .ok_or("Missing session_id")?
        .to_string();

    save_session(app, &session_id);
    Ok(session_id)
}

/// Validate current session.
pub async fn validate_session(app: &AppHandle) -> Result<bool, String> {
    let session_id = match get_session(app) {
        Some(id) => id,
        None => return Ok(false),
    };

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/auth/me", api_url()))
        .header("Cookie", format!("session={}", session_id))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(response.status().is_success())
}

/// Log out the current session.
pub async fn logout(app: &AppHandle) -> Result<(), String> {
    let session_id = match get_session(app) {
        Some(id) => id,
        None => return Ok(()),
    };

    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{}/auth/logout", api_url()))
        .header("Cookie", format!("session={}", session_id))
        .send()
        .await;

    crate::auth::clear_session(app);
    Ok(())
}

/// Fetch user's calendars.
pub async fn fetch_my_calendars(app: &AppHandle) -> Result<Vec<CalendarWithRole>, String> {
    let (client, session_id) = client_with_session(app)?;

    let response = client
        .get(format!("{}/api/calendars/me", api_url()))
        .header("Cookie", format!("session={}", session_id))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("UNAUTHORIZED".to_string());
    }
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format_http_error("fetch calendars", status, &body));
    }

    response.json().await.map_err(|e| e.to_string())
}

/// Fetch calendar entries for a date range.
pub async fn fetch_entries(
    app: &AppHandle,
    calendar_id: &str,
    highlighted_day: &str,
    before: Option<i32>,
    after: Option<i32>,
) -> Result<Vec<ServerDay>, String> {
    let (client, session_id) = client_with_session(app)?;

    let mut url = format!(
        "{}/api/entries?calendar_id={}&highlighted_day={}",
        api_url(),
        calendar_id,
        highlighted_day
    );
    if let Some(b) = before {
        url.push_str(&format!("&before={}", b));
    }
    if let Some(a) = after {
        url.push_str(&format!("&after={}", a));
    }

    let response = client
        .get(&url)
        .header("Cookie", format!("session={}", session_id))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Failed: {}", response.status()));
    }

    response.json().await.map_err(|e| e.to_string())
}

/// Convert payload to form-urlencoded format.
fn payload_to_form(payload: &CreateEntryPayload) -> Vec<(String, String)> {
    let mut form: Vec<(String, String)> = vec![
        ("calendar_id".to_string(), payload.calendar_id.clone()),
        ("title".to_string(), payload.title.clone()),
        ("date".to_string(), payload.date.clone()),
    ];

    if let Some(ref start_time) = payload.start_time {
        form.push(("start_time".to_string(), start_time.clone()));
    }
    if let Some(ref end_time) = payload.end_time {
        form.push(("end_time".to_string(), end_time.clone()));
    }
    if let Some(all_day) = payload.all_day {
        form.push(("all_day".to_string(), all_day.to_string()));
    }
    if let Some(ref description) = payload.description {
        form.push(("description".to_string(), description.clone()));
    }
    if let Some(ref entry_type) = payload.entry_type {
        form.push(("entry_type".to_string(), entry_type.clone()));
    }

    form
}

/// Create a new entry.
pub async fn create_entry(
    app: &AppHandle,
    payload: CreateEntryPayload,
) -> Result<ServerEntry, String> {
    let (client, session_id) = client_with_session(app)?;
    let form_data = payload_to_form(&payload);

    let response = client
        .post(format!("{}/api/entries", api_url()))
        .header("Cookie", format!("session={}", session_id))
        .form(&form_data)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format_http_error("create entry", status, &body));
    }

    response.json().await.map_err(|e| e.to_string())
}

/// Update an existing entry.
pub async fn update_entry(
    app: &AppHandle,
    id: &str,
    payload: CreateEntryPayload,
) -> Result<ServerEntry, String> {
    let (client, session_id) = client_with_session(app)?;
    let form_data = payload_to_form(&payload);

    let response = client
        .put(format!("{}/api/entries/{}", api_url(), id))
        .header("Cookie", format!("session={}", session_id))
        .form(&form_data)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format_http_error("update entry", status, &body));
    }

    response.json().await.map_err(|e| e.to_string())
}

/// Delete an entry.
pub async fn delete_entry(app: &AppHandle, id: &str) -> Result<(), String> {
    let (client, session_id) = client_with_session(app)?;

    let response = client
        .delete(format!("{}/api/entries/{}", api_url(), id))
        .header("Cookie", format!("session={}", session_id))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Failed to delete entry: {}", response.status()));
    }

    Ok(())
}

/// Toggle a task's completed status.
pub async fn toggle_entry(app: &AppHandle, id: &str) -> Result<ServerEntry, String> {
    let (client, session_id) = client_with_session(app)?;

    let response = client
        .patch(format!("{}/api/entries/{}/toggle", api_url(), id))
        .header("Cookie", format!("session={}", session_id))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Failed to toggle entry: {}", response.status()));
    }

    response.json().await.map_err(|e| e.to_string())
}

/// Fetch a single entry by ID.
pub async fn fetch_entry(app: &AppHandle, id: &str) -> Result<ServerEntry, String> {
    let (client, session_id) = client_with_session(app)?;

    let response = client
        .get(format!("{}/api/entries/{}", api_url(), id))
        .header("Cookie", format!("session={}", session_id))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch entry: {}", response.status()));
    }

    response.json().await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_http_error_with_body() {
        let error = format_http_error(
            "create entry",
            reqwest::StatusCode::BAD_REQUEST,
            "Invalid date format",
        );
        assert_eq!(
            error,
            "Failed to create entry: 400 Bad Request - Invalid date format"
        );
    }

    #[test]
    fn test_format_http_error_empty_body() {
        let error = format_http_error("update entry", reqwest::StatusCode::NOT_FOUND, "");
        assert_eq!(error, "Failed to update entry: 404 Not Found - ");
    }

    #[test]
    fn test_format_http_error_internal_server_error() {
        let error = format_http_error(
            "delete entry",
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            "Database connection failed",
        );
        assert_eq!(
            error,
            "Failed to delete entry: 500 Internal Server Error - Database connection failed"
        );
    }

    #[test]
    fn test_format_http_error_unauthorized() {
        let error = format_http_error(
            "fetch calendars",
            reqwest::StatusCode::UNAUTHORIZED,
            "Session expired",
        );
        assert_eq!(
            error,
            "Failed to fetch calendars: 401 Unauthorized - Session expired"
        );
    }

    #[test]
    fn test_format_http_error_exchange_auth_code() {
        let error = format_http_error(
            "exchange auth code",
            reqwest::StatusCode::BAD_REQUEST,
            "Invalid authorization code",
        );
        assert_eq!(
            error,
            "Failed to exchange auth code: 400 Bad Request - Invalid authorization code"
        );
    }
}
