//! Development-only handlers for hot-reload.
//!
//! Only available when DEV_MODE environment variable is set.

use std::convert::Infallible;
use std::path::Path as FsPath;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};

use calendsync_ssr::{SsrPool, SsrPoolConfig};

use crate::state::AppState;

/// Request body for build error endpoint.
#[derive(serde::Deserialize)]
pub struct BuildErrorRequest {
    pub error: String,
}

/// Request body for reload endpoint with optional manifest comparison.
#[derive(serde::Deserialize, Default)]
pub struct ReloadRequest {
    /// Previous CSS filename for change detection.
    #[serde(default)]
    pub prev_css: Option<String>,
    /// Previous server JS filename for change detection.
    #[serde(default)]
    pub prev_server_js: Option<String>,
    /// Previous client JS filename for change detection.
    #[serde(default)]
    pub prev_client_js: Option<String>,
}

/// POST /_dev/reload - Reload SSR bundle (dev mode only).
///
/// Reads the new manifest, detects what changed, and takes minimal action:
/// - "none": No changes, skip reload entirely
/// - "css_only": Only CSS changed, hot-swap without pool swap or page reload
/// - "client_only": Only client JS changed, page reload without pool swap
/// - "full": Server JS changed, swap pool and reload page
///
/// This endpoint is only available when DEV_MODE environment variable is set.
#[axum::debug_handler]
pub async fn reload_ssr(
    State(state): State<AppState>,
    Json(body): Json<ReloadRequest>,
) -> impl IntoResponse {
    let frontend_dir = FsPath::new(env!("CARGO_MANIFEST_DIR")).join("../frontend");
    let manifest_path = frontend_dir.join("manifest.json");

    // Read manifest from disk
    let manifest_str = match std::fs::read_to_string(&manifest_path) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to read manifest: {}", e)
                })),
            );
        }
    };

    // Parse manifest
    let manifest: serde_json::Value = match serde_json::from_str(&manifest_str) {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to parse manifest: {}", e)
                })),
            );
        }
    };

    // Get all bundle names from manifest
    let new_server_js = manifest
        .get("calendsync.js")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync-server.js");

    let new_client_js = manifest
        .get("calendsync-client.js")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync-client.js");

    let new_css = manifest
        .get("calendsync.css")
        .and_then(|v| v.as_str())
        .unwrap_or("calendsync.css");

    // Handle first request (no prev values) - log and treat as full reload
    let is_first_request =
        body.prev_css.is_none() && body.prev_server_js.is_none() && body.prev_client_js.is_none();

    if is_first_request {
        tracing::info!("First reload request - no previous state, performing full reload");
    }

    // Determine what changed
    let css_changed = body.prev_css.as_ref().is_some_and(|prev| prev != new_css);
    let server_js_changed = body
        .prev_server_js
        .as_ref()
        .is_some_and(|prev| prev != new_server_js);
    let client_js_changed = body
        .prev_client_js
        .as_ref()
        .is_some_and(|prev| prev != new_client_js);

    // Determine action based on change matrix
    let change_type = if is_first_request {
        "full"
    } else {
        match (server_js_changed, client_js_changed, css_changed) {
            (false, false, false) => "none",
            (false, false, true) => "css_only",
            (false, true, _) => "client_only",
            (true, _, _) => "full",
        }
    };

    let bundle_path = frontend_dir.join("dist").join(new_server_js);

    // Only swap pool if server JS changed (or first request)
    if change_type == "full" {
        // Create pool config
        let worker_count = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);

        let pool_config = match SsrPoolConfig::with_defaults(worker_count) {
            Ok(c) => c,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("Invalid pool config: {}", e)
                    })),
                );
            }
        };

        // Create new pool
        let new_pool = match SsrPool::new(pool_config, &bundle_path) {
            Ok(p) => p,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("Failed to create SSR pool: {}", e)
                    })),
                );
            }
        };

        // Swap the pool
        state.swap_ssr_pool(new_pool).await;
    }

    // Signal browsers based on change type
    match change_type {
        "none" => {
            tracing::info!("No changes detected, skipping reload");
        }
        "css_only" => {
            state.signal_dev_css_reload(new_css.to_string());
            tracing::info!(css = new_css, "CSS hot-swapped (no pool swap)");
        }
        "client_only" => {
            state.signal_dev_reload();
            tracing::info!(
                client_js = new_client_js,
                "Client JS changed (no pool swap)"
            );
        }
        _ => {
            // "full"
            state.signal_dev_reload();
            tracing::info!(
                server_js = new_server_js,
                "Server JS changed, SSR pool swapped"
            );
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "bundle": bundle_path.display().to_string(),
            "css": new_css,
            "server_js": new_server_js,
            "client_js": new_client_js,
            "change_type": change_type
        })),
    )
}

/// POST /_dev/error - Report build error (dev mode only).
///
/// Called by xtask when the frontend build fails. Broadcasts the error
/// to all connected browsers for display in an error overlay.
#[axum::debug_handler]
pub async fn report_build_error(
    State(state): State<AppState>,
    Json(body): Json<BuildErrorRequest>,
) -> impl IntoResponse {
    state.signal_dev_error(body.error);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true
        })),
    )
}

/// GET /_dev/events - SSE endpoint for dev reload signals.
///
/// Browsers connect to this endpoint to receive reload and build error notifications.
/// - "reload" event: Signals browser to refresh the page
/// - "css-reload" event: Signals browser to hot-swap CSS without full reload
/// - "build-error" event: Signals browser to display error overlay
pub async fn dev_events_sse(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let mut reload_rx = state.subscribe_dev_reload();
    let mut error_rx = state.subscribe_dev_error();
    let mut css_reload_rx = state.subscribe_dev_css_reload();
    let mut shutdown_rx = state.subscribe_shutdown();

    let stream = async_stream::stream! {
        // Send initial connection event
        yield Ok(Event::default().event("connected").data("{}"));

        loop {
            tokio::select! {
                // Reload signal received
                Ok(()) = reload_rx.recv() => {
                    yield Ok(Event::default().event("reload").data("{}"));
                }
                // CSS reload signal received
                Ok(css) = css_reload_rx.recv() => {
                    let data = serde_json::json!({ "filename": css.filename });
                    yield Ok(Event::default().event("css-reload").data(data.to_string()));
                }
                // Build error received
                Ok(error) = error_rx.recv() => {
                    let data = serde_json::json!({ "error": error.error });
                    yield Ok(Event::default().event("build-error").data(data.to_string()));
                }
                // Shutdown signal - close connection
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ============================================================================
// Dev Annotations
// ============================================================================

/// A UI annotation created in dev mode for Claude Code collaboration.
///
/// Captures element metadata, computed styles, and optional screenshot
/// so Claude Code can understand exactly which element the developer
/// is referring to.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DevAnnotation {
    pub id: String,
    pub timestamp: String,
    pub selector: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component_name: Option<String>,
    pub tag_name: String,
    pub text_content: String,
    pub note: String,
    pub bounding_box: BoundingBox,
    pub computed_styles: ComputedStyles,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    pub resolved: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_summary: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BoundingBox {
    pub top: f64,
    pub left: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComputedStyles {
    pub color: String,
    pub background_color: String,
    pub font_size: String,
    pub font_family: String,
    pub padding: String,
    pub margin: String,
    pub width: String,
    pub height: String,
    pub display: String,
    pub position: String,
}

/// Request body for creating a new annotation.
#[derive(Debug, serde::Deserialize)]
pub struct CreateAnnotationRequest {
    pub selector: String,
    #[serde(default)]
    pub component_name: Option<String>,
    pub tag_name: String,
    #[serde(default)]
    pub text_content: String,
    pub note: String,
    pub bounding_box: BoundingBox,
    pub computed_styles: ComputedStyles,
    #[serde(default)]
    pub screenshot: Option<String>,
}

impl CreateAnnotationRequest {
    /// Converts the request into a `DevAnnotation` with server-assigned fields.
    pub fn into_annotation(self) -> DevAnnotation {
        DevAnnotation {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            selector: self.selector,
            component_name: self.component_name,
            tag_name: self.tag_name,
            text_content: self.text_content,
            note: self.note,
            bounding_box: self.bounding_box,
            computed_styles: self.computed_styles,
            screenshot: self.screenshot,
            resolved: false,
            resolution_summary: None,
        }
    }
}

/// Request body for resolving an annotation.
#[derive(Debug, serde::Deserialize)]
pub struct ResolveAnnotationRequest {
    pub summary: String,
}

/// Finds an annotation by ID in a slice. Returns the index if found.
pub fn find_annotation_index(annotations: &[DevAnnotation], id: &str) -> Option<usize> {
    annotations.iter().position(|a| a.id == id)
}

/// Summary of annotation counts.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AnnotationSummary {
    pub total: usize,
    pub resolved: usize,
    pub unresolved: usize,
}

/// Counts resolved and unresolved annotations.
pub fn count_annotations_summary(annotations: &[DevAnnotation]) -> AnnotationSummary {
    let resolved = annotations.iter().filter(|a| a.resolved).count();
    AnnotationSummary {
        total: annotations.len(),
        resolved,
        unresolved: annotations.len() - resolved,
    }
}

// --- Handlers ---

use crate::handlers::error::AppError;

/// Acquire a read lock on the dev annotations, returning a 500 if poisoned.
fn read_annotations(
    state: &AppState,
) -> Result<std::sync::RwLockReadGuard<'_, Vec<DevAnnotation>>, AppError> {
    state
        .dev_annotations
        .read()
        .map_err(|e| AppError(anyhow::anyhow!("dev_annotations lock poisoned: {e}")))
}

/// Acquire a write lock on the dev annotations, returning a 500 if poisoned.
fn write_annotations(
    state: &AppState,
) -> Result<std::sync::RwLockWriteGuard<'_, Vec<DevAnnotation>>, AppError> {
    state
        .dev_annotations
        .write()
        .map_err(|e| AppError(anyhow::anyhow!("dev_annotations lock poisoned: {e}")))
}

#[axum::debug_handler]
pub async fn create_annotation(
    State(state): State<AppState>,
    Json(body): Json<CreateAnnotationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let annotation = body.into_annotation();
    let id = annotation.id.clone();
    write_annotations(&state)?.push(annotation);
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

#[axum::debug_handler]
pub async fn list_annotations(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let annotations = read_annotations(&state)?;
    let summary = count_annotations_summary(&annotations);
    Ok(Json(serde_json::json!({
        "annotations": *annotations,
        "summary": summary,
    })))
}

#[axum::debug_handler]
pub async fn get_annotation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, AppError> {
    let annotations = read_annotations(&state)?;
    match find_annotation_index(&annotations, &id) {
        Some(idx) => Ok(Json(annotations[idx].clone()).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

#[axum::debug_handler]
pub async fn resolve_annotation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ResolveAnnotationRequest>,
) -> Result<axum::response::Response, AppError> {
    let mut annotations = write_annotations(&state)?;
    match find_annotation_index(&annotations, &id) {
        Some(idx) => {
            annotations[idx].resolved = true;
            annotations[idx].resolution_summary = Some(body.summary);
            Ok((
                StatusCode::OK,
                Json(serde_json::json!({ "resolved": true })),
            )
                .into_response())
        }
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

#[axum::debug_handler]
pub async fn delete_annotation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let mut annotations = write_annotations(&state)?;
    match find_annotation_index(&annotations, &id) {
        Some(idx) => {
            annotations.remove(idx);
            Ok(StatusCode::NO_CONTENT)
        }
        None => Ok(StatusCode::NOT_FOUND),
    }
}

#[axum::debug_handler]
pub async fn clear_annotations(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let mut annotations = write_annotations(&state)?;
    let count = annotations.len();
    annotations.clear();
    Ok(Json(serde_json::json!({ "cleared": count })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bounding_box() -> BoundingBox {
        BoundingBox {
            top: 100.0,
            left: 200.0,
            width: 300.0,
            height: 50.0,
        }
    }

    fn sample_computed_styles() -> ComputedStyles {
        ComputedStyles {
            color: "rgb(0, 0, 0)".to_string(),
            background_color: "rgb(255, 255, 255)".to_string(),
            font_size: "16px".to_string(),
            font_family: "Inter, sans-serif".to_string(),
            padding: "8px".to_string(),
            margin: "0px".to_string(),
            width: "300px".to_string(),
            height: "50px".to_string(),
            display: "block".to_string(),
            position: "relative".to_string(),
        }
    }

    fn sample_annotation(id: &str, resolved: bool) -> DevAnnotation {
        DevAnnotation {
            id: id.to_string(),
            timestamp: "2024-01-15T10:00:00Z".to_string(),
            selector: "div.calendar > h1".to_string(),
            component_name: Some("CalendarHeader".to_string()),
            tag_name: "h1".to_string(),
            text_content: "January 2024".to_string(),
            note: "Font size too small on mobile".to_string(),
            bounding_box: sample_bounding_box(),
            computed_styles: sample_computed_styles(),
            screenshot: None,
            resolved,
            resolution_summary: if resolved {
                Some("Increased font size to 24px".to_string())
            } else {
                None
            },
        }
    }

    #[test]
    fn test_find_annotation_index_found() {
        let annotations = vec![
            sample_annotation("aaa", false),
            sample_annotation("bbb", false),
            sample_annotation("ccc", true),
        ];
        assert_eq!(find_annotation_index(&annotations, "bbb"), Some(1));
    }

    #[test]
    fn test_find_annotation_index_not_found() {
        let annotations = vec![sample_annotation("aaa", false)];
        assert_eq!(find_annotation_index(&annotations, "zzz"), None);
    }

    #[test]
    fn test_find_annotation_index_empty() {
        let annotations: Vec<DevAnnotation> = vec![];
        assert_eq!(find_annotation_index(&annotations, "aaa"), None);
    }

    #[test]
    fn test_count_annotations_summary_mixed() {
        let annotations = vec![
            sample_annotation("a", false),
            sample_annotation("b", true),
            sample_annotation("c", false),
            sample_annotation("d", true),
        ];
        assert_eq!(
            count_annotations_summary(&annotations),
            AnnotationSummary {
                total: 4,
                resolved: 2,
                unresolved: 2,
            }
        );
    }

    #[test]
    fn test_count_annotations_summary_empty() {
        let annotations: Vec<DevAnnotation> = vec![];
        assert_eq!(
            count_annotations_summary(&annotations),
            AnnotationSummary {
                total: 0,
                resolved: 0,
                unresolved: 0,
            }
        );
    }

    #[test]
    fn test_count_annotations_summary_all_unresolved() {
        let annotations = vec![sample_annotation("a", false), sample_annotation("b", false)];
        assert_eq!(
            count_annotations_summary(&annotations),
            AnnotationSummary {
                total: 2,
                resolved: 0,
                unresolved: 2,
            }
        );
    }

    #[test]
    fn test_create_annotation_request_into_annotation() {
        let request = CreateAnnotationRequest {
            selector: "div.test".to_string(),
            component_name: Some("TestComponent".to_string()),
            tag_name: "div".to_string(),
            text_content: "Hello".to_string(),
            note: "Fix this".to_string(),
            bounding_box: sample_bounding_box(),
            computed_styles: sample_computed_styles(),
            screenshot: None,
        };

        let annotation = request.into_annotation();

        assert_eq!(annotation.selector, "div.test");
        assert_eq!(annotation.component_name, Some("TestComponent".to_string()));
        assert!(!annotation.resolved);
        assert!(annotation.resolution_summary.is_none());
        assert!(!annotation.id.is_empty());
        assert!(!annotation.timestamp.is_empty());
    }

    #[test]
    fn test_dev_annotation_serde_roundtrip() {
        let annotation = sample_annotation("test-id", false);
        let json = serde_json::to_string(&annotation).unwrap();
        let deserialized: DevAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-id");
        assert_eq!(deserialized.note, "Font size too small on mobile");
        assert!(!deserialized.resolved);
    }
}
