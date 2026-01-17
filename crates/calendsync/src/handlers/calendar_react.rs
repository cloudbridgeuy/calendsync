//! React SSR handler for `/calendar/{calendar_id}`.
//!
//! Uses the SSR worker pool from `calendsync_ssr` to render the React calendar.

use std::collections::BTreeMap;
use std::sync::Arc;

use calendsync_ssr::SsrPool;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use axum::response::Redirect;

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use super::flash::{redirect_with_flash, FlashMessage};

use crate::context::RequestContext;
use chrono::Local;
use uuid::Uuid;

use calendsync_core::calendar::{CalendarEntry, User};
use calendsync_core::storage::DateRange;
use calendsync_ssr::{sanitize_error, SsrConfig, SsrError};

/// User info subset for SSR props (serialized to frontend).
///
/// This is intentionally a **security projection** of `User` - it only includes
/// fields safe to expose to the client (name, email). Sensitive fields like
/// `id`, `provider`, `provider_subject`, and timestamps are excluded.
///
/// Matches the TypeScript `UserInfo` interface in `frontend/src/calendsync/types.ts`.
#[derive(serde::Serialize)]
struct SsrUserInfo {
    name: String,
    email: String,
}

impl From<&User> for SsrUserInfo {
    fn from(user: &User) -> Self {
        Self {
            name: user.name.clone(),
            email: user.email.clone(),
        }
    }
}

use super::entries::entry_to_server_entry;
use crate::state::AppState;

/// Check if dev mode with auto-refresh is enabled.
/// DEV_MODE enables dev features, DEV_NO_AUTO_REFRESH disables browser auto-refresh.
fn is_dev_mode() -> bool {
    std::env::var("DEV_MODE").is_ok() && std::env::var("DEV_NO_AUTO_REFRESH").is_err()
}

/// Query parameters for the entry modal route.
#[derive(serde::Deserialize, Default)]
pub struct EntryModalQuery {
    /// Entry ID for edit mode (optional - if missing, we're in create mode)
    pub entry_id: Option<Uuid>,
}

/// Bundle URLs for client JS and CSS.
struct BundleUrls {
    client_js: String,
    css: String,
}

/// Get bundle URLs from the manifest.
///
/// In dev mode (DEV_MODE env var set), reads manifest from disk to pick up
/// new hashed filenames after hot-reload. In production, uses compiled-in manifest.
fn get_bundle_urls() -> BundleUrls {
    let manifest = get_manifest();

    let client_bundle_name = manifest
        .get("calendsync-client.js")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync-client.js");

    let css_bundle_name = manifest
        .get("calendsync.css")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync.css");

    BundleUrls {
        client_js: format!("/dist/{client_bundle_name}"),
        css: format!("/dist/{css_bundle_name}"),
    }
}

/// Get the manifest JSON, reading from disk in dev mode or using compiled-in manifest.
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

/// Group entries by date into ServerDay format for a date range.
/// Creates entries for all dates in the range, even if they have no entries.
fn entries_to_server_days(
    entries: &[&CalendarEntry],
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
) -> Vec<serde_json::Value> {
    let mut days_map: BTreeMap<chrono::NaiveDate, Vec<serde_json::Value>> = BTreeMap::new();

    // Initialize all dates in the range
    let mut current = start;
    while current <= end {
        days_map.insert(current, Vec::new());
        current += chrono::Duration::days(1);
    }

    // Add entries - use start_date for grouping
    // (Frontend will expand multi-day entries)
    for entry in entries {
        if entry.start_date >= start && entry.start_date <= end {
            let server_entry = entry_to_server_entry(entry);
            days_map
                .entry(entry.start_date)
                .or_default()
                .push(server_entry);
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

/// Generate simple HTML page for service unavailable states (e.g., SSR initializing).
/// Includes auto-refresh hint to encourage user to retry.
fn initializing_html(title: &str, message: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>{title}</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
            background: #f9fafb;
        }}
        .container {{
            text-align: center;
            padding: 2rem;
        }}
        h1 {{
            color: #374151;
            margin-bottom: 0.5rem;
        }}
        p {{
            color: #6b7280;
            margin-bottom: 1.5rem;
        }}
        .spinner {{
            width: 40px;
            height: 40px;
            border: 3px solid #e5e7eb;
            border-top-color: #3b82f6;
            border-radius: 50%;
            animation: spin 1s linear infinite;
            margin: 0 auto 1rem;
        }}
        @keyframes spin {{
            to {{ transform: rotate(360deg); }}
        }}
        .retry-button {{
            padding: 0.75rem 1.5rem;
            background: #3b82f6;
            color: white;
            border: none;
            border-radius: 0.5rem;
            cursor: pointer;
            font-size: 1rem;
        }}
        .retry-button:hover {{
            background: #2563eb;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="spinner"></div>
        <h1>{title}</h1>
        <p>{message}</p>
        <button class="retry-button" onclick="location.reload()">Refresh</button>
    </div>
</body>
</html>"#
    )
}

/// Generate error HTML with client-side fallback.
/// When dev_mode is true, includes auto-refresh script with retry logic for self-healing.
fn error_html(
    error: &str,
    calendar_id: &str,
    client_bundle_url: &str,
    css_bundle_url: &str,
    dev_mode: bool,
) -> String {
    let dev_script = if dev_mode {
        r#"
    <!-- Dev mode auto-refresh: connect to SSE and reload on signal -->
    <script>
    (function() {
        var es = new EventSource('/_dev/events');
        var retryCount = 0;
        var maxRetries = 10;

        es.addEventListener('reload', function() {
            console.log('[Dev] Reload signal received, refreshing...');
            location.reload();
        });

        es.addEventListener('connected', function() {
            console.log('[Dev] Auto-refresh connected');
            retryCount = 0; // Reset on successful connection
        });

        es.onerror = function() {
            retryCount++;
            if (retryCount <= maxRetries) {
                console.log('[Dev] Connection failed, retry ' + retryCount + '/' + maxRetries);
                setTimeout(function() {
                    location.reload();
                }, 2000);
            } else {
                console.log('[Dev] Max retries reached, stopping auto-refresh');
            }
        };
    })();
    </script>"#
    } else {
        ""
    };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Calendar</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link rel="stylesheet" href="{css_bundle_url}">
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
    </script>{dev_script}
</body>
</html>"#,
        dev_script = dev_script
    )
}

/// SSR handler for `/calendar/{calendar_id}` (with auth check).
///
/// Renders the React calendar server-side using the SSR worker pool.
/// Redirects to login if not authenticated.
/// Redirects to user's first calendar with flash message if no access.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
#[axum::debug_handler]
pub async fn calendar_react_ssr(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(calendar_id): Path<Uuid>,
) -> Response {
    tracing::debug!(request_id = %ctx.request_id, "Rendering calendar");

    // Auth check: redirect to login if not authenticated
    let user = match ctx.user {
        Some(u) => u,
        None => {
            let path = format!("/calendar/{}", calendar_id);
            let return_to = urlencoding::encode(&path);
            return Redirect::to(&format!("/login?return_to={}", return_to)).into_response();
        }
    };

    // Membership check: verify user has access to this calendar
    if let Some(auth) = &state.auth {
        match auth.memberships.get_membership(calendar_id, user.id).await {
            Ok(Some(_membership)) => {
                // User has access, continue to render
            }
            Ok(None) => {
                // No membership - redirect to user's first calendar with flash message
                tracing::warn!(
                    user_id = %user.id,
                    calendar_id = %calendar_id,
                    request_id = %ctx.request_id,
                    "User attempted to access calendar without membership"
                );
                return redirect_to_first_calendar_with_flash(&state, user.id).await;
            }
            Err(e) => {
                tracing::error!(error = %e, request_id = %ctx.request_id, "Failed to check calendar membership");
                // On error, still try to redirect to user's calendar
                return redirect_to_first_calendar_with_flash(&state, user.id).await;
            }
        }
    }

    let user_info = Some(SsrUserInfo::from(&user));
    calendar_react_ssr_impl(state, calendar_id, user_info).await
}

/// SSR handler for `/calendar/{calendar_id}` (no auth).
///
/// Renders the React calendar server-side using the SSR worker pool.
#[cfg(not(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock")))]
#[axum::debug_handler]
pub async fn calendar_react_ssr(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(calendar_id): Path<Uuid>,
) -> Response {
    tracing::debug!(request_id = %ctx.request_id, "Rendering calendar");
    calendar_react_ssr_impl(state, calendar_id, None).await
}

/// Redirect user to their first calendar with a flash message about no access.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
async fn redirect_to_first_calendar_with_flash(state: &AppState, user_id: Uuid) -> Response {
    let flash = FlashMessage::error("You don't have access to the requested calendar");

    let auth = match &state.auth {
        Some(auth) => auth,
        None => {
            tracing::error!("Auth state not initialized");
            return redirect_with_flash("/login", flash);
        }
    };

    match auth.memberships.get_calendars_for_user(user_id).await {
        Ok(calendars) if !calendars.is_empty() => {
            let first_calendar_id = calendars[0].0.id;
            redirect_with_flash(&format!("/calendar/{}", first_calendar_id), flash)
        }
        Ok(_) => {
            // No calendars found - redirect to login
            tracing::warn!(user_id = %user_id, "User has no calendars");
            redirect_with_flash("/login", flash)
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get user calendars");
            redirect_with_flash("/login", flash)
        }
    }
}

/// Implementation of SSR handler for `/calendar/{calendar_id}`.
///
/// Renders the React calendar server-side using the SSR worker pool.
/// If the calendar doesn't exist, shows an error page.
///
/// # Design Note: user_info Parameter
///
/// The `user_info` parameter is passed explicitly rather than stored in `AppState`
/// because of axum's architecture separation:
///
/// - **`AppState`**: Holds application-scoped resources shared across all requests
///   (repositories, SSR pool, event channels). Cloned for each request.
///
/// - **User info**: Request-scoped data extracted by authentication middleware via
///   the `OptionalUser` extractor. Different for each request based on session.
///
/// This pattern is standard in axum - shared services in state, per-request auth
/// data from extractors. The public handlers extract the user and pass it here.
async fn calendar_react_ssr_impl(
    state: AppState,
    calendar_id: Uuid,
    user_info: Option<SsrUserInfo>,
) -> Response {
    // Get bundle URLs and dev mode early (needed for error pages)
    let urls = get_bundle_urls();
    let dev_mode = is_dev_mode();

    // Validate calendar exists
    let calendar = match state.calendar_repo.get_calendar(calendar_id).await {
        Ok(Some(cal)) => cal,
        Ok(None) => {
            tracing::warn!(calendar_id = %calendar_id, "Calendar not found");
            return Html(error_html(
                "Calendar not found",
                &calendar_id.to_string(),
                &urls.client_js,
                &urls.css,
                dev_mode,
            ))
            .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch calendar");
            return Html(error_html(
                &format!("Failed to load calendar: {e}"),
                &calendar_id.to_string(),
                &urls.client_js,
                &urls.css,
                dev_mode,
            ))
            .into_response();
        }
    };

    // Check if SSR pool is available (async for lazy initialization support)
    let Some(ssr_pool) = state.get_ssr_pool().await else {
        // SSR still initializing - return 503 with retry hint
        tracing::info!("SSR pool still initializing, returning 503");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [("Retry-After", "5")],
            Html(initializing_html(
                "Calendar Loading",
                "The calendar view is initializing. Please refresh in a few seconds.",
            )),
        )
            .into_response();
    };

    // Get today's date as the highlighted day
    let today = Local::now().date_naive();
    let highlighted_day = today.to_string();

    // Calculate date range (before=365, after=365)
    let start = today - chrono::Duration::days(365);
    let end = today + chrono::Duration::days(365);

    // Fetch entries for the date range
    let date_range = match DateRange::new(start, end) {
        Ok(range) => range,
        Err(e) => {
            tracing::error!(error = %e, "Invalid date range");
            return Html(error_html(
                "Invalid date range",
                &calendar_id.to_string(),
                &urls.client_js,
                &urls.css,
                dev_mode,
            ))
            .into_response();
        }
    };

    let entries = match state
        .entry_repo
        .get_entries_by_calendar(calendar.id, date_range)
        .await
    {
        Ok(entries) => entries,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch entries");
            return Html(error_html(
                &format!("Failed to load entries: {e}"),
                &calendar_id.to_string(),
                &urls.client_js,
                &urls.css,
                dev_mode,
            ))
            .into_response();
        }
    };

    let entry_refs: Vec<&CalendarEntry> = entries.iter().collect();
    let days = entries_to_server_days(&entry_refs, start, end);

    // Build initial data for SSR
    let initial_data = serde_json::json!({
        "calendarId": calendar_id.to_string(),
        "highlightedDay": highlighted_day,
        "days": days,
        "clientBundleUrl": urls.client_js,
        "cssBundleUrl": urls.css,
        "controlPlaneUrl": "",
        "devMode": dev_mode,
        "user": user_info,
    });

    // Create SSR config (with payload size validation)
    let config = match SsrConfig::new(initial_data) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create SSR config");
            return Html(error_html(
                &sanitize_error(&SsrError::Core(e)),
                &calendar_id.to_string(),
                &urls.client_js,
                &urls.css,
                dev_mode,
            ))
            .into_response();
        }
    };

    // Render using SSR pool
    render_with_ssr_pool(&ssr_pool, config, &calendar_id.to_string(), &urls, dev_mode).await
}

/// SSR handler for `/calendar/{calendar_id}/entry` (with auth check).
///
/// Renders the React calendar with entry modal open for creating or editing.
/// Redirects to login if not authenticated.
/// Redirects to user's first calendar with flash message if no access.
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
#[axum::debug_handler]
pub async fn calendar_react_ssr_entry(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(calendar_id): Path<Uuid>,
    Query(query): Query<EntryModalQuery>,
) -> Response {
    tracing::debug!(request_id = %ctx.request_id, "Rendering calendar entry modal");

    // Auth check: redirect to login if not authenticated
    let user = match ctx.user {
        Some(u) => u,
        None => {
            // Include entry_id in return_to if present (for edit mode)
            let return_to = match query.entry_id {
                Some(entry_id) => format!("/calendar/{}/entry?entry_id={}", calendar_id, entry_id),
                None => format!("/calendar/{}/entry", calendar_id),
            };
            let return_to_encoded = urlencoding::encode(&return_to);
            return Redirect::to(&format!("/login?return_to={}", return_to_encoded))
                .into_response();
        }
    };

    // Membership check: verify user has access to this calendar
    if let Some(auth) = &state.auth {
        match auth.memberships.get_membership(calendar_id, user.id).await {
            Ok(Some(_membership)) => {
                // User has access, continue to render
            }
            Ok(None) => {
                // No membership - redirect to user's first calendar with flash message
                tracing::warn!(
                    user_id = %user.id,
                    calendar_id = %calendar_id,
                    request_id = %ctx.request_id,
                    "User attempted to access calendar entry without membership"
                );
                return redirect_to_first_calendar_with_flash(&state, user.id).await;
            }
            Err(e) => {
                tracing::error!(error = %e, request_id = %ctx.request_id, "Failed to check calendar membership");
                return redirect_to_first_calendar_with_flash(&state, user.id).await;
            }
        }
    }

    let user_info = Some(SsrUserInfo::from(&user));
    calendar_react_ssr_entry_impl(state, calendar_id, query, user_info).await
}

/// SSR handler for `/calendar/{calendar_id}/entry` (no auth).
///
/// Renders the React calendar with entry modal open for creating or editing.
#[cfg(not(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock")))]
#[axum::debug_handler]
pub async fn calendar_react_ssr_entry(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(calendar_id): Path<Uuid>,
    Query(query): Query<EntryModalQuery>,
) -> Response {
    tracing::debug!(request_id = %ctx.request_id, "Rendering calendar entry modal");
    calendar_react_ssr_entry_impl(state, calendar_id, query, None).await
}

/// Implementation of SSR handler for `/calendar/{calendar_id}/entry`.
///
/// Renders the React calendar with entry modal open for creating or editing.
/// - Without `entry_id` query param: Create mode (modal open with highlighted day)
/// - With `entry_id=uuid` query param: Edit mode (modal open with entry data)
///
/// See `calendar_react_ssr_impl` for why `user_info` is a parameter (request-scoped
/// auth data vs application-scoped state).
async fn calendar_react_ssr_entry_impl(
    state: AppState,
    calendar_id: Uuid,
    query: EntryModalQuery,
    user_info: Option<SsrUserInfo>,
) -> Response {
    // Get bundle URLs and dev mode early (needed for error pages)
    let urls = get_bundle_urls();
    let dev_mode = is_dev_mode();

    // Validate calendar exists
    let calendar = match state.calendar_repo.get_calendar(calendar_id).await {
        Ok(Some(cal)) => cal,
        Ok(None) => {
            tracing::warn!(calendar_id = %calendar_id, "Calendar not found");
            return Html(error_html(
                "Calendar not found",
                &calendar_id.to_string(),
                &urls.client_js,
                &urls.css,
                dev_mode,
            ))
            .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch calendar");
            return Html(error_html(
                &format!("Failed to load calendar: {e}"),
                &calendar_id.to_string(),
                &urls.client_js,
                &urls.css,
                dev_mode,
            ))
            .into_response();
        }
    };

    // Check if SSR pool is available (async for lazy initialization support)
    let Some(ssr_pool) = state.get_ssr_pool().await else {
        // SSR still initializing - return 503 with retry hint
        tracing::info!("SSR pool still initializing, returning 503");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [("Retry-After", "5")],
            Html(initializing_html(
                "Calendar Loading",
                "The calendar view is initializing. Please refresh in a few seconds.",
            )),
        )
            .into_response();
    };

    // Get today's date as the highlighted day
    let today = Local::now().date_naive();
    let highlighted_day = today.to_string();

    // Calculate date range (before=365, after=365)
    let start = today - chrono::Duration::days(365);
    let end = today + chrono::Duration::days(365);

    // Fetch entries for the date range
    let date_range = match DateRange::new(start, end) {
        Ok(range) => range,
        Err(e) => {
            tracing::error!(error = %e, "Invalid date range");
            return Html(error_html(
                "Invalid date range",
                &calendar_id.to_string(),
                &urls.client_js,
                &urls.css,
                dev_mode,
            ))
            .into_response();
        }
    };

    let entries = match state
        .entry_repo
        .get_entries_by_calendar(calendar.id, date_range)
        .await
    {
        Ok(entries) => entries,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch entries");
            return Html(error_html(
                &format!("Failed to load entries: {e}"),
                &calendar_id.to_string(),
                &urls.client_js,
                &urls.css,
                dev_mode,
            ))
            .into_response();
        }
    };

    let entry_refs: Vec<&CalendarEntry> = entries.iter().collect();
    let days = entries_to_server_days(&entry_refs, start, end);

    // Build modal state based on query params
    let modal = if let Some(entry_id) = query.entry_id {
        // Edit mode: look up the entry from repository
        match state.entry_repo.get_entry(entry_id).await {
            Ok(Some(entry)) => serde_json::json!({
                "mode": "edit",
                "entryId": entry_id.to_string(),
                "entry": entry_to_server_entry(&entry),
            }),
            Ok(None) => {
                // Entry not found - return 404
                return (
                    StatusCode::NOT_FOUND,
                    Html(error_html(
                        "Entry not found",
                        &calendar_id.to_string(),
                        &urls.client_js,
                        &urls.css,
                        dev_mode,
                    )),
                )
                    .into_response();
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to fetch entry for edit");
                return Html(error_html(
                    &format!("Failed to load entry: {e}"),
                    &calendar_id.to_string(),
                    &urls.client_js,
                    &urls.css,
                    dev_mode,
                ))
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

    // Build initial data for SSR with modal state
    let initial_data = serde_json::json!({
        "calendarId": calendar_id.to_string(),
        "highlightedDay": highlighted_day,
        "days": days,
        "clientBundleUrl": urls.client_js,
        "cssBundleUrl": urls.css,
        "controlPlaneUrl": "",
        "modal": modal,
        "devMode": dev_mode,
        "user": user_info,
    });

    // Create SSR config (with payload size validation)
    let config = match SsrConfig::new(initial_data) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create SSR config");
            return Html(error_html(
                &sanitize_error(&SsrError::Core(e)),
                &calendar_id.to_string(),
                &urls.client_js,
                &urls.css,
                dev_mode,
            ))
            .into_response();
        }
    };

    // Render using SSR pool
    render_with_ssr_pool(&ssr_pool, config, &calendar_id.to_string(), &urls, dev_mode).await
}

/// Helper to render with SSR pool and handle errors consistently.
async fn render_with_ssr_pool(
    ssr_pool: &Arc<SsrPool>,
    config: SsrConfig,
    calendar_id: &str,
    urls: &BundleUrls,
    dev_mode: bool,
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
                    &urls.client_js,
                    &urls.css,
                    dev_mode,
                )),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "SSR render failed");
            Html(error_html(
                &sanitize_error(&e),
                calendar_id,
                &urls.client_js,
                &urls.css,
                dev_mode,
            ))
            .into_response()
        }
    }
}
