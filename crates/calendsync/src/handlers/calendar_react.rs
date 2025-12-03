//! React SSR handler for `/calendar/{calendar_id}`.
//!
//! Uses the SSR worker pool from `calendsync_ssr` to render the React calendar.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use chrono::Local;
use uuid::Uuid;

use calendsync_core::calendar::{filter_entries, CalendarEntry};
use calendsync_ssr::{sanitize_error, SsrConfig, SsrError};

use crate::state::AppState;

/// Get the client bundle URL from the manifest.
fn get_client_bundle_url() -> String {
    let manifest_str = include_str!("../../../frontend/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(manifest_str).unwrap_or(serde_json::json!({}));

    let client_bundle_name = manifest
        .get("calendar-react-client.js")
        .and_then(|v| v.as_str())
        .unwrap_or("calendar-react-client.js");

    format!("/dist/{client_bundle_name}")
}

/// Convert CalendarEntry to the ServerEntry format expected by the frontend.
fn entry_to_server_entry(entry: &CalendarEntry) -> serde_json::Value {
    use calendsync_core::calendar::EntryKind;

    let (kind, completed, is_multi_day, is_all_day, is_timed, is_task) = match &entry.kind {
        EntryKind::AllDay => ("all-day", false, false, true, false, false),
        EntryKind::Timed { .. } => ("timed", false, false, false, true, false),
        EntryKind::Task { completed } => ("task", *completed, false, false, false, true),
        EntryKind::MultiDay { .. } => ("multi-day", false, true, false, false, false),
    };

    let start_time = entry
        .kind
        .start_time()
        .map(|t| t.format("%H:%M").to_string());
    let end_time = entry.kind.end_time().map(|t| t.format("%H:%M").to_string());
    let multi_day_start = entry
        .kind
        .multi_day_start()
        .map(|d| d.format("%b %d").to_string());
    let multi_day_end = entry
        .kind
        .multi_day_end()
        .map(|d| d.format("%b %d").to_string());
    let multi_day_start_date = entry.kind.multi_day_start().map(|d| d.to_string());
    let multi_day_end_date = entry.kind.multi_day_end().map(|d| d.to_string());

    serde_json::json!({
        "id": entry.id.to_string(),
        "calendarId": entry.calendar_id.to_string(),
        "kind": kind,
        "completed": completed,
        "isMultiDay": is_multi_day,
        "isAllDay": is_all_day,
        "isTimed": is_timed,
        "isTask": is_task,
        "title": entry.title,
        "description": entry.description,
        "location": entry.location,
        "color": entry.color,
        "date": entry.date.to_string(),
        "startTime": start_time,
        "endTime": end_time,
        "multiDayStart": multi_day_start,
        "multiDayEnd": multi_day_end,
        "multiDayStartDate": multi_day_start_date,
        "multiDayEndDate": multi_day_end_date,
    })
}

/// Group entries by date into ServerDay format for a date range.
/// Creates entries for all dates in the range, even if they have no entries.
fn entries_to_server_days(
    entries: &[&CalendarEntry],
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
) -> Vec<serde_json::Value> {
    use std::collections::BTreeMap;

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

/// Query parameters for the calendar entries API.
#[derive(serde::Deserialize)]
pub struct CalendarEntriesQuery {
    /// Calendar ID
    #[allow(dead_code)]
    pub calendar_id: Uuid,
    /// Center date (ISO 8601: YYYY-MM-DD)
    pub highlighted_day: chrono::NaiveDate,
    /// Number of days before highlighted_day (default: 365)
    #[serde(default = "default_before")]
    pub before: i64,
    /// Number of days after highlighted_day (default: 365)
    #[serde(default = "default_after")]
    pub after: i64,
}

fn default_before() -> i64 {
    365
}
fn default_after() -> i64 {
    365
}

/// API handler for fetching calendar entries in ServerDay[] format.
/// Used by the React calendar client for data fetching.
///
/// GET /api/calendar-entries?calendar_id=...&highlighted_day=...&before=3&after=3
///
/// NOTE: This generates mock entries on-the-fly for the requested date range.
/// In production, this would query a database.
#[axum::debug_handler]
pub async fn calendar_entries_api(
    State(_state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<CalendarEntriesQuery>,
) -> axum::Json<Vec<serde_json::Value>> {
    let start = query.highlighted_day - chrono::Duration::days(query.before);
    let end = query.highlighted_day + chrono::Duration::days(query.after);

    // Generate mock entries for the requested date range
    let demo_calendar_id =
        Uuid::parse_str("fc9f55e0-dd5e-4988-a5f3-9ae520859857").unwrap_or_else(|_| Uuid::new_v4());

    let entries = crate::mock_data::generate_mock_entries(demo_calendar_id, query.highlighted_day);
    let filtered: Vec<&CalendarEntry> = filter_entries(&entries, None, Some(start), Some(end));
    let days = entries_to_server_days(&filtered, start, end);

    axum::Json(days)
}

/// Generate error HTML with client-side fallback.
fn error_html(error: &str, calendar_id: &str, client_bundle_url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Calendar</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link rel="stylesheet" href="/dist/calendar-react.css">
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
#[axum::debug_handler]
pub async fn calendar_react_ssr(
    State(state): State<AppState>,
    Path(calendar_id): Path<Uuid>,
) -> Response {
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
    let config = match SsrConfig::new(serde_json::json!({
        "initialData": initial_data,
    })) {
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
    match ssr_pool.render(config).await {
        Ok(html) => Html(html).into_response(),
        Err(SsrError::Overloaded { retry_after_secs }) => {
            // Return 503 with Retry-After header
            (
                StatusCode::SERVICE_UNAVAILABLE,
                [("Retry-After", retry_after_secs.to_string())],
                Html(error_html(
                    &sanitize_error(&SsrError::Overloaded { retry_after_secs }),
                    &calendar_id.to_string(),
                    &client_bundle_url,
                )),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "SSR render failed");
            Html(error_html(
                &sanitize_error(&e),
                &calendar_id.to_string(),
                &client_bundle_url,
            ))
            .into_response()
        }
    }
}
