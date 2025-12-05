//! React SSR handler for `/calendar/{calendar_id}`.
//!
//! Uses the SSR worker pool from `calendsync_ssr` to render the React calendar.

use std::collections::BTreeMap;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use chrono::Local;
use uuid::Uuid;

use calendsync_core::calendar::{filter_entries, CalendarEntry};
use calendsync_ssr::{sanitize_error, SsrConfig, SsrError};

use super::entries::entry_to_server_entry;
use crate::state::AppState;

/// Query parameters for the entry modal route.
#[derive(serde::Deserialize, Default)]
pub struct EntryModalQuery {
    /// Entry ID for edit mode (optional - if missing, we're in create mode)
    pub entry_id: Option<Uuid>,
}

/// Get the client bundle URL from the manifest.
fn get_client_bundle_url() -> String {
    let manifest_str = include_str!("../../../frontend/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(manifest_str).unwrap_or(serde_json::json!({}));

    let client_bundle_name = manifest
        .get("calendsync-client.js")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync-client.js");

    format!("/dist/{client_bundle_name}")
}

/// Group entries by date into ServerDay format for a date range.
/// Creates entries for all dates in the range, even if they have no entries.
fn entries_to_server_days(
    entries: &[&CalendarEntry],
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
) -> Vec<serde_json::Value> {
    // Build a map of entries by date
    let mut days_map: BTreeMap<chrono::NaiveDate, Vec<serde_json::Value>> = BTreeMap::new();

    // Initialize all dates in the range with empty vectors
    let mut current = start;
    while current <= end {
        days_map.insert(current, Vec::new());
        current += chrono::Duration::days(1);
    }

    // Add entries to their respective dates
    for entry in entries {
        if entry.date >= start && entry.date <= end {
            let server_entry = entry_to_server_entry(entry);
            days_map.entry(entry.date).or_default().push(server_entry);
        }
    }

    days_map
        .into_iter()
        .map(|(date, entries)| {
            serde_json::json!({
                "date": date.to_string(),
                "entries": entries,
            })
        })
        .collect()
}

/// Generate error HTML with client-side fallback.
fn error_html(error: &str, calendar_id: &str, client_bundle_url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Calendar</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link rel="stylesheet" href="/dist/calendsync.css">
    <style>
        .error-container {{
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            padding: 2rem;
            text-align: center;
        }}
        .error-message {{ color: #dc2626; margin-bottom: 1rem; }}
        .retry-button {{
            padding: 0.75rem 1.5rem;
            background: #3b82f6;
            color: white;
            border: none;
            border-radius: 0.5rem;
            cursor: pointer;
        }}
    </style>
</head>
<body>
    <div class="error-container" id="error">
        <h1>Unable to load calendar</h1>
        <p class="error-message">{error}</p>
        <button class="retry-button" onclick="location.reload()">Retry</button>
    </div>
    <!-- Fallback: try client-side render -->
    <div id="root" style="display:none"></div>
    <script>
        window.__INITIAL_DATA__ = {{
            calendarId: "{calendar_id}",
            highlightedDay: new Date().toISOString().split('T')[0],
            days: [],
            clientBundleUrl: "{client_bundle_url}",
            controlPlaneUrl: ""
        }};
    </script>
    <script type="module" src="{client_bundle_url}" onerror="document.getElementById('error').style.display='flex'"></script>
    <script>
        // If client bundle loads, hide error and show app
        window.addEventListener('load', () => {{
            if (window.__CALENDAR_LOADED__) {{
                document.getElementById('error').style.display = 'none';
                document.getElementById('root').style.display = 'block';
            }}
        }});
    </script>
</body>
</html>"#
    )
}

/// SSR handler for `/calendar/{calendar_id}`.
///
/// Renders the React calendar server-side using the SSR worker pool.
/// If the calendar doesn't exist, redirects to the default calendar.
#[axum::debug_handler]
pub async fn calendar_react_ssr(
    State(state): State<AppState>,
    Path(calendar_id): Path<Uuid>,
) -> Response {
    // Validate calendar exists, redirect to default if not found
    let calendar_exists = state
        .calendars
        .read()
        .expect("Failed to acquire read lock")
        .contains_key(&calendar_id);

    if !calendar_exists {
        tracing::warn!(
            calendar_id = %calendar_id,
            "Calendar not found, redirecting to default"
        );

        // Try to redirect to default calendar
        if let Some(default_id) = state.default_calendar_id() {
            return Redirect::to(&format!("/calendar/{default_id}")).into_response();
        }

        // No calendars available at all
        let client_bundle_url = get_client_bundle_url();
        return Html(error_html(
            "No calendars available",
            &calendar_id.to_string(),
            &client_bundle_url,
        ))
        .into_response();
    }

    // Check if SSR pool is available
    let Some(ssr_pool) = &state.ssr_pool else {
        tracing::error!("SSR pool not initialized");
        let client_bundle_url = get_client_bundle_url();
        return Html(error_html(
            "SSR not available",
            &calendar_id.to_string(),
            &client_bundle_url,
        ))
        .into_response();
    };

    // Get today's date as the highlighted day
    let today = Local::now().date_naive();
    let highlighted_day = today.to_string();

    // Calculate date range (before=365, after=365)
    let start = today - chrono::Duration::days(365);
    let end = today + chrono::Duration::days(365);

    // Fetch entries for the date range (scope to drop lock before await)
    let days = {
        let entries_store = state.entries.read().expect("Failed to acquire read lock");
        let all_entries: Vec<CalendarEntry> = entries_store.values().cloned().collect();
        let filtered: Vec<&CalendarEntry> =
            filter_entries(&all_entries, None, Some(start), Some(end));
        entries_to_server_days(&filtered, start, end)
    };

    // Get client bundle URL
    let client_bundle_url = get_client_bundle_url();

    // Build initial data for SSR
    let initial_data = serde_json::json!({
        "calendarId": calendar_id.to_string(),
        "highlightedDay": highlighted_day,
        "days": days,
        "clientBundleUrl": client_bundle_url,
        "controlPlaneUrl": "",
    });

    // Create SSR config (with payload size validation)
    let config = match SsrConfig::new(initial_data) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create SSR config");
            return Html(error_html(
                &sanitize_error(&SsrError::Core(e)),
                &calendar_id.to_string(),
                &client_bundle_url,
            ))
            .into_response();
        }
    };

    // Render using SSR pool
    render_with_ssr_pool(
        ssr_pool,
        config,
        &calendar_id.to_string(),
        &client_bundle_url,
    )
    .await
}

/// SSR handler for `/calendar/{calendar_id}/entry`.
///
/// Renders the React calendar with entry modal open for creating or editing.
/// - Without `entry_id` query param: Create mode (modal open with highlighted day)
/// - With `entry_id=uuid` query param: Edit mode (modal open with entry data)
/// If the calendar doesn't exist, redirects to the default calendar.
#[axum::debug_handler]
pub async fn calendar_react_ssr_entry(
    State(state): State<AppState>,
    Path(calendar_id): Path<Uuid>,
    Query(query): Query<EntryModalQuery>,
) -> Response {
    // Validate calendar exists, redirect to default if not found
    let calendar_exists = state
        .calendars
        .read()
        .expect("Failed to acquire read lock")
        .contains_key(&calendar_id);

    if !calendar_exists {
        tracing::warn!(
            calendar_id = %calendar_id,
            "Calendar not found, redirecting to default"
        );

        // Try to redirect to default calendar (with /entry path)
        if let Some(default_id) = state.default_calendar_id() {
            // Preserve entry_id query param if present
            let redirect_url = if let Some(entry_id) = query.entry_id {
                format!("/calendar/{default_id}/entry?entry_id={entry_id}")
            } else {
                format!("/calendar/{default_id}/entry")
            };
            return Redirect::to(&redirect_url).into_response();
        }

        // No calendars available at all
        let client_bundle_url = get_client_bundle_url();
        return Html(error_html(
            "No calendars available",
            &calendar_id.to_string(),
            &client_bundle_url,
        ))
        .into_response();
    }

    // Check if SSR pool is available
    let Some(ssr_pool) = &state.ssr_pool else {
        tracing::error!("SSR pool not initialized");
        let client_bundle_url = get_client_bundle_url();
        return Html(error_html(
            "SSR not available",
            &calendar_id.to_string(),
            &client_bundle_url,
        ))
        .into_response();
    };

    // Get today's date as the highlighted day
    let today = Local::now().date_naive();
    let highlighted_day = today.to_string();

    // Calculate date range (before=365, after=365)
    let start = today - chrono::Duration::days(365);
    let end = today + chrono::Duration::days(365);

    // Get client bundle URL
    let client_bundle_url = get_client_bundle_url();

    // Fetch entries and optionally the specific entry for edit mode
    let (days, modal) = {
        let entries_store = state.entries.read().expect("Failed to acquire read lock");
        let all_entries: Vec<CalendarEntry> = entries_store.values().cloned().collect();
        let filtered: Vec<&CalendarEntry> =
            filter_entries(&all_entries, None, Some(start), Some(end));
        let days = entries_to_server_days(&filtered, start, end);

        // Build modal state based on query params
        let modal = if let Some(entry_id) = query.entry_id {
            // Edit mode: look up the entry
            match entries_store.get(&entry_id) {
                Some(entry) => serde_json::json!({
                    "mode": "edit",
                    "entryId": entry_id.to_string(),
                    "entry": entry_to_server_entry(entry),
                }),
                None => {
                    // Entry not found - return 404
                    return (
                        StatusCode::NOT_FOUND,
                        Html(error_html(
                            "Entry not found",
                            &calendar_id.to_string(),
                            &client_bundle_url,
                        )),
                    )
                        .into_response();
                }
            }
        } else {
            // Create mode: pre-fill with highlighted day
            serde_json::json!({
                "mode": "create",
                "defaultDate": highlighted_day,
            })
        };

        (days, modal)
    };

    // Build initial data for SSR with modal state
    let initial_data = serde_json::json!({
        "calendarId": calendar_id.to_string(),
        "highlightedDay": highlighted_day,
        "days": days,
        "clientBundleUrl": client_bundle_url,
        "controlPlaneUrl": "",
        "modal": modal,
    });

    // Create SSR config (with payload size validation)
    let config = match SsrConfig::new(initial_data) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create SSR config");
            return Html(error_html(
                &sanitize_error(&SsrError::Core(e)),
                &calendar_id.to_string(),
                &client_bundle_url,
            ))
            .into_response();
        }
    };

    // Render using SSR pool
    render_with_ssr_pool(
        ssr_pool,
        config,
        &calendar_id.to_string(),
        &client_bundle_url,
    )
    .await
}

/// Helper to render with SSR pool and handle errors consistently.
async fn render_with_ssr_pool(
    ssr_pool: &calendsync_ssr::SsrPool,
    config: SsrConfig,
    calendar_id: &str,
    client_bundle_url: &str,
) -> Response {
    match ssr_pool.render(config).await {
        Ok(html) => Html(html).into_response(),
        Err(SsrError::Overloaded { retry_after_secs }) => {
            // Return 503 with Retry-After header
            (
                StatusCode::SERVICE_UNAVAILABLE,
                [("Retry-After", retry_after_secs.to_string())],
                Html(error_html(
                    &sanitize_error(&SsrError::Overloaded { retry_after_secs }),
                    calendar_id,
                    client_bundle_url,
                )),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "SSR render failed");
            Html(error_html(
                &sanitize_error(&e),
                calendar_id,
                client_bundle_url,
            ))
            .into_response()
        }
    }
}
