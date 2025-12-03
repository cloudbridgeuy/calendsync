//! React SSR handler for `/calendar/{calendar_id}`.
//!
//! Uses deno_core to server-side render the React calendar with prerender API.

use std::cell::RefCell;
use std::sync::OnceLock;

use axum::{
    extract::{Path, State},
    response::Html,
};
use chrono::Local;
use deno_core::{extension, op2, JsRuntime, RuntimeOptions};
use tokio::sync::oneshot;
use uuid::Uuid;

use calendsync_core::calendar::{filter_entries, CalendarEntry};

use crate::state::AppState;

// Cache the server bundle in memory after first read
static SERVER_BUNDLE: OnceLock<String> = OnceLock::new();

// Thread-local storage for the rendered HTML (used within SSR thread)
thread_local! {
    static RENDERED_HTML: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Custom op to receive HTML from JavaScript
#[op2(fast)]
fn op_set_html(#[string] html: String) {
    RENDERED_HTML.with(|cell| {
        *cell.borrow_mut() = Some(html);
    });
}

// Define the extension with the op
extension!(react_ssr_ext, ops = [op_set_html]);

/// Web API polyfills required for React 19 prerender to run in deno_core.
fn get_polyfills(config_json: &str) -> String {
    let node_env = std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string());
    let control_plane_url = std::env::var("CONTROL_PLANE_URL")
        .unwrap_or_else(|_| "http://calendsync.localhost:8000".to_string());

    format!(
        r#"
// SSR Configuration - injected by Rust
globalThis.__SSR_CONFIG__ = {config_json};

// Process polyfill (Node.js compatibility)
globalThis.process = {{
    env: {{
        NODE_ENV: '{node_env}',
        CONTROL_PLANE_URL: '{control_plane_url}',
    }},
    nextTick: (fn) => queueMicrotask(fn),
}};

// Console polyfill - forward JS logs to Rust stdout
globalThis.console = {{
    log: (...args) => Deno.core.print('[JS] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n', false),
    error: (...args) => Deno.core.print('[JS ERROR] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n', true),
    warn: (...args) => Deno.core.print('[JS WARN] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n', false),
    info: (...args) => Deno.core.print('[JS] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n', false),
    debug: () => {{}},
}};

// Performance polyfill for timing
const performanceStart = Date.now();
globalThis.performance = {{
    now: () => Date.now() - performanceStart,
}};

// MessageChannel polyfill - React uses this for scheduling
class MessageChannelPolyfill {{
    constructor() {{
        this.port1 = {{
            postMessage: () => {{
                if (this.port2.onmessage) {{
                    queueMicrotask(this.port2.onmessage);
                }}
            }},
            onmessage: null,
        }};
        this.port2 = {{
            postMessage: () => {{
                if (this.port1.onmessage) {{
                    queueMicrotask(this.port1.onmessage);
                }}
            }},
            onmessage: null,
        }};
    }}
}}
globalThis.MessageChannel = MessageChannelPolyfill;

// TextEncoder/TextDecoder polyfills - used by React for string encoding
class TextEncoderPolyfill {{
    encode(str) {{
        const utf8 = unescape(encodeURIComponent(str));
        const result = new Uint8Array(utf8.length);
        for (let i = 0; i < utf8.length; i++) {{
            result[i] = utf8.charCodeAt(i);
        }}
        return result;
    }}
    encodeInto(str, dest) {{
        const encoded = this.encode(str);
        const len = Math.min(encoded.length, dest.length);
        dest.set(encoded.subarray(0, len));
        return {{ read: str.length, written: len }};
    }}
}}
globalThis.TextEncoder = TextEncoderPolyfill;

class TextDecoderPolyfill {{
    constructor(label = 'utf-8') {{
        this.encoding = label.toLowerCase();
    }}
    decode(input) {{
        if (!input) return '';
        const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
        let result = '';
        for (let i = 0; i < bytes.length; i++) {{
            result += String.fromCharCode(bytes[i]);
        }}
        return decodeURIComponent(escape(result));
    }}
}}
globalThis.TextDecoder = TextDecoderPolyfill;

// ReadableStream polyfill for React 19 prerender
class ReadableStreamPolyfill {{
    constructor(underlyingSource) {{
        this._source = underlyingSource;
        this._started = false;
        this._done = false;
        this._chunks = [];
        this._controller = {{
            enqueue: (chunk) => {{ this._chunks.push(chunk); }},
            close: () => {{ this._done = true; }},
            error: (e) => {{ this._error = e; }},
            desiredSize: 1,
        }};
    }}

    getReader() {{
        const self = this;
        return {{
            async read() {{
                // Check for errors
                if (self._error) {{
                    throw self._error;
                }}

                // If we have buffered chunks, return one
                if (self._chunks.length > 0) {{
                    return {{ done: false, value: self._chunks.shift() }};
                }}

                // Check if stream is done
                if (self._done) {{
                    return {{ done: true, value: undefined }};
                }}

                // Start the stream if not started
                if (!self._started) {{
                    self._started = true;
                    if (self._source.start) {{
                        await self._source.start(self._controller);
                    }}
                }}

                // Return any chunks enqueued during start
                if (self._chunks.length > 0) {{
                    return {{ done: false, value: self._chunks.shift() }};
                }}

                // Keep pulling until we get data or stream closes
                while (!self._done && self._chunks.length === 0) {{
                    if (self._source.pull) {{
                        await self._source.pull(self._controller);
                    }} else {{
                        // No pull function and no data means stream is done
                        break;
                    }}
                }}

                // Check for errors again
                if (self._error) {{
                    throw self._error;
                }}

                // Return chunk if available
                if (self._chunks.length > 0) {{
                    return {{ done: false, value: self._chunks.shift() }};
                }}

                // Stream is done
                return {{ done: true, value: undefined }};
            }},
            releaseLock() {{}},
        }};
    }}
}}
globalThis.ReadableStream = ReadableStreamPolyfill;

// WritableStream polyfill (minimal, for React)
class WritableStreamPolyfill {{
    constructor(underlyingSink) {{
        this._sink = underlyingSink;
    }}
    getWriter() {{
        const self = this;
        return {{
            write(chunk) {{
                if (self._sink && self._sink.write) {{
                    return self._sink.write(chunk);
                }}
            }},
            close() {{
                if (self._sink && self._sink.close) {{
                    return self._sink.close();
                }}
            }},
            releaseLock() {{}},
        }};
    }}
}}
globalThis.WritableStream = WritableStreamPolyfill;

// TransformStream polyfill (minimal, for React)
class TransformStreamPolyfill {{
    constructor(transformer) {{
        this._transformer = transformer;
        this.readable = new ReadableStreamPolyfill({{
            start: () => {{}},
            pull: () => {{}},
        }});
        this.writable = new WritableStreamPolyfill({{
            write: () => {{}},
            close: () => {{}},
        }});
    }}
}}
globalThis.TransformStream = TransformStreamPolyfill;
"#
    )
}

/// Load the server bundle from disk (cached after first load).
fn load_server_bundle() -> anyhow::Result<&'static str> {
    // Try to get the cached bundle first
    if let Some(bundle) = SERVER_BUNDLE.get() {
        return Ok(bundle.as_str());
    }

    // Load and cache the bundle
    let manifest_str = include_str!("../../../frontend/manifest.json");
    let manifest: serde_json::Value = serde_json::from_str(manifest_str)?;

    // Get the server bundle filename from manifest
    let server_bundle_name = manifest
        .get("calendar-react.js")
        .and_then(|v| v.as_str())
        .unwrap_or("calendar-react-server.js");

    let dist_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../frontend/dist")
        .join(server_bundle_name);

    let content = std::fs::read_to_string(&dist_path)
        .map_err(|e| anyhow::anyhow!("Failed to read server bundle at {dist_path:?}: {e}"))?;

    // Store and return (race condition is fine - same content)
    let _ = SERVER_BUNDLE.set(content);
    Ok(SERVER_BUNDLE.get().unwrap().as_str())
}

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

/// Run React SSR in a dedicated thread.
async fn run_react_ssr(config_json: String) -> anyhow::Result<String> {
    let js_code = load_server_bundle()?;

    // Create runtime with our custom extension
    let mut runtime = JsRuntime::new(RuntimeOptions {
        extensions: vec![react_ssr_ext::init()],
        ..Default::default()
    });

    // Generate polyfills with config
    let polyfills = get_polyfills(&config_json);

    // Inject polyfills, then execute React bundle
    runtime.execute_script("<polyfills>", polyfills)?;
    runtime.execute_script("<react-ssr>", js_code.to_string())?;

    // Run event loop for async operations
    runtime.run_event_loop(Default::default()).await?;

    // Retrieve the rendered HTML
    RENDERED_HTML
        .with(|cell| cell.borrow_mut().take())
        .ok_or_else(|| anyhow::anyhow!("No HTML was rendered"))
}

/// SSR handler for `/calendar/{calendar_id}`.
///
/// Renders the React calendar server-side and returns HTML.
#[axum::debug_handler]
pub async fn calendar_react_ssr(
    State(state): State<AppState>,
    Path(calendar_id): Path<Uuid>,
) -> Html<String> {
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

    let config = serde_json::json!({
        "initialData": initial_data,
    });

    let config_json = serde_json::to_string(&config).expect("Failed to serialize config");

    // Spawn SSR in a dedicated thread because deno_core's JsRuntime is not Send
    let (tx, rx) = oneshot::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let _ = tx.send(rt.block_on(run_react_ssr(config_json)));
    });

    match rx.await {
        Ok(Ok(html)) => Html(html),
        Ok(Err(e)) => {
            tracing::error!(error = %e, "React SSR failed");
            Html(format!(
                r#"<!DOCTYPE html>
<html>
<head><title>Error</title></head>
<body><h1>Error rendering calendar</h1><pre>{e}</pre></body>
</html>"#
            ))
        }
        Err(e) => {
            tracing::error!(error = %e, "SSR channel error");
            Html(format!(
                r#"<!DOCTYPE html>
<html>
<head><title>Error</title></head>
<body><h1>Internal Error</h1><pre>{e}</pre></body>
</html>"#
            ))
        }
    }
}
