//! Annotation CRUD handlers, watch endpoint, and annotation SSE.
//!
//! Imperative Shell — thin handlers that delegate to the SQLite store
//! and broadcast events.

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use tokio::sync::broadcast;

use crate::handlers::error::AppError;
use crate::state::AppState;

use super::get_store;
use super::types::{
    count_annotations_summary, validate_status_transition, AccessibilityInfo, AnnotationIntent,
    AnnotationSeverity, AnnotationStatus, BoundingBox, ComputedStyles, DevAnnotation,
    DevAnnotationEvent, ThreadAuthor, ThreadMessage,
};

// ============================================================================
// Request / response types
// ============================================================================

#[derive(Debug, serde::Deserialize)]
pub struct CreateAnnotationRequest {
    pub url: String,
    pub element_path: String,
    pub comment: String,
    pub tag_name: String,
    #[serde(default)]
    pub text_content: String,
    #[serde(default)]
    pub component_name: Option<String>,
    #[serde(default = "default_intent")]
    pub intent: AnnotationIntent,
    #[serde(default = "default_severity")]
    pub severity: AnnotationSeverity,
    #[serde(default)]
    pub selected_text: Option<String>,
    #[serde(default)]
    pub nearby_text: Option<String>,
    #[serde(default)]
    pub css_classes: Vec<String>,
    pub bounding_box: BoundingBox,
    pub computed_styles: ComputedStyles,
    #[serde(default)]
    pub accessibility: Option<AccessibilityInfo>,
    #[serde(default)]
    pub full_path: Option<String>,
    #[serde(default)]
    pub screenshot: Option<String>,
}

fn default_intent() -> AnnotationIntent {
    AnnotationIntent::Fix
}
fn default_severity() -> AnnotationSeverity {
    AnnotationSeverity::Suggestion
}

impl CreateAnnotationRequest {
    /// Convert to a `DevAnnotation` with server-assigned fields.
    pub fn into_annotation(self, session_id: String) -> DevAnnotation {
        DevAnnotation {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            tag_name: self.tag_name,
            text_content: self.text_content,
            bounding_box: self.bounding_box,
            computed_styles: self.computed_styles,
            screenshot: self.screenshot,
            component_name: self.component_name,
            session_id,
            url: self.url,
            element_path: self.element_path,
            comment: self.comment,
            intent: self.intent,
            severity: self.severity,
            status: AnnotationStatus::Pending,
            selected_text: self.selected_text,
            nearby_text: self.nearby_text,
            css_classes: self.css_classes,
            accessibility: self.accessibility,
            full_path: self.full_path,
            is_fixed: false,
            resolved_at: None,
            resolved_by: None,
            thread: vec![],
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ListAnnotationsParams {
    pub session_id: Option<String>,
    pub status: Option<String>,
}

/// Body accepted for API compatibility (summary is not stored in the DB).
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct ResolveRequest {
    pub summary: String,
}

/// Body accepted for API compatibility (reason is not stored in the DB).
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct DismissRequest {
    pub reason: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ThreadMessageRequest {
    pub message: String,
    pub author: ThreadAuthor,
}

#[derive(serde::Deserialize)]
pub struct WatchParams {
    /// Timeout in seconds (default: 30, max: 120).
    pub timeout: Option<u64>,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /_dev/annotations
#[axum::debug_handler]
pub async fn create_annotation(
    State(state): State<AppState>,
    Json(body): Json<CreateAnnotationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let store = get_store(&state)?;

    // Auto-create session from URL
    let session = store.find_or_create_session(&body.url).await?;
    let annotation = body.into_annotation(session.id);
    let id = annotation.id.clone();

    store.create_annotation(&annotation).await?;

    let _ = state
        .dev_annotation_tx
        .send(DevAnnotationEvent::Created(annotation));

    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

/// GET /_dev/annotations
#[axum::debug_handler]
pub async fn list_annotations(
    State(state): State<AppState>,
    Query(params): Query<ListAnnotationsParams>,
) -> Result<impl IntoResponse, AppError> {
    let store = get_store(&state)?;

    let annotations = store
        .list_filtered(params.session_id.as_deref(), params.status.as_deref())
        .await?;
    let summary = count_annotations_summary(&annotations);

    Ok(Json(serde_json::json!({
        "annotations": annotations,
        "summary": summary,
    })))
}

/// GET /_dev/annotations/pending
#[axum::debug_handler]
pub async fn list_pending_annotations(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let store = get_store(&state)?;
    let annotations: Vec<DevAnnotation> = store.list_pending().await?;
    Ok(Json(serde_json::json!({ "annotations": annotations })))
}

/// GET /_dev/annotations/{id}
#[axum::debug_handler]
pub async fn get_annotation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, AppError> {
    let store = get_store(&state)?;
    match store.get_annotation(&id).await? {
        Some(ann) => Ok(Json::<DevAnnotation>(ann).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

/// Shared logic for status transition handlers: validate, update, broadcast.
async fn transition_annotation(
    state: &AppState,
    id: &str,
    target_status: AnnotationStatus,
    resolved_by: Option<&str>,
) -> Result<(), AppError> {
    let store = get_store(state)?;

    let annotation = store
        .get_annotation(id)
        .await?
        .ok_or_else(|| AppError(anyhow::anyhow!("annotation not found")))?;

    validate_status_transition(annotation.status, target_status)
        .map_err(|e| AppError(anyhow::anyhow!("{e}")))?;

    store.update_status(id, target_status, resolved_by).await?;

    if let Some(updated) = store.get_annotation(id).await? {
        let _ = state
            .dev_annotation_tx
            .send(DevAnnotationEvent::Updated(updated));
    }

    Ok(())
}

/// PATCH /_dev/annotations/{id}/acknowledge
#[axum::debug_handler]
pub async fn acknowledge_annotation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, AppError> {
    transition_annotation(&state, &id, AnnotationStatus::Acknowledged, None).await?;
    Ok(StatusCode::OK.into_response())
}

/// PATCH /_dev/annotations/{id}/resolve
#[axum::debug_handler]
pub async fn resolve_annotation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(_body): Json<ResolveRequest>,
) -> Result<axum::response::Response, AppError> {
    transition_annotation(&state, &id, AnnotationStatus::Resolved, Some("agent")).await?;
    Ok(StatusCode::OK.into_response())
}

/// PATCH /_dev/annotations/{id}/dismiss
#[axum::debug_handler]
pub async fn dismiss_annotation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(_body): Json<DismissRequest>,
) -> Result<axum::response::Response, AppError> {
    transition_annotation(&state, &id, AnnotationStatus::Dismissed, Some("agent")).await?;
    Ok(StatusCode::OK.into_response())
}

/// DELETE /_dev/annotations/{id}
#[axum::debug_handler]
pub async fn delete_annotation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let store = get_store(&state)?;

    if store.delete_annotation(&id).await? {
        let _ = state
            .dev_annotation_tx
            .send(DevAnnotationEvent::Deleted { id });
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

/// DELETE /_dev/annotations
#[axum::debug_handler]
pub async fn clear_annotations(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let store = get_store(&state)?;
    let count: usize = store.clear_all().await?;
    Ok(Json(serde_json::json!({ "cleared": count })))
}

/// POST /_dev/annotations/{id}/thread
#[axum::debug_handler]
pub async fn add_thread_message(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ThreadMessageRequest>,
) -> Result<impl IntoResponse, AppError> {
    let store = get_store(&state)?;

    // Verify annotation exists
    store
        .get_annotation(&id)
        .await?
        .ok_or_else(|| AppError(anyhow::anyhow!("annotation not found")))?;

    let msg = ThreadMessage {
        id: uuid::Uuid::new_v4().to_string(),
        annotation_id: id,
        message: body.message,
        author: body.author,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    store.add_thread_message(&msg).await?;

    let _ = state
        .dev_annotation_tx
        .send(DevAnnotationEvent::ThreadMessage(msg.clone()));

    Ok((StatusCode::CREATED, Json(msg)))
}

// ============================================================================
// Watch (long-poll)
// ============================================================================

/// GET /_dev/annotations/watch
#[axum::debug_handler]
pub async fn watch_annotations(
    State(state): State<AppState>,
    Query(params): Query<WatchParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let store = get_store(&state)?;
    let timeout_secs = params.timeout.unwrap_or(30).min(120);

    // Check for existing pending annotations first
    let pending: Vec<DevAnnotation> = store.list_pending().await?;
    if !pending.is_empty() {
        return Ok(Json(serde_json::json!({
            "annotations": pending,
            "timed_out": false,
        })));
    }

    // No pending — wait for an event or timeout
    let mut event_rx = state.dev_annotation_tx.subscribe();
    let mut shutdown_rx = state.subscribe_shutdown();
    let sleep = tokio::time::sleep(Duration::from_secs(timeout_secs));
    tokio::pin!(sleep);

    let timed_out = loop {
        tokio::select! {
            result = event_rx.recv() => {
                match result {
                    Ok(DevAnnotationEvent::Created(_)) => break false,
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break true,
                }
            }
            () = &mut sleep => break true,
            _ = shutdown_rx.recv() => break true,
        }
    };

    let pending: Vec<DevAnnotation> = store.list_pending().await?;
    Ok(Json(serde_json::json!({
        "annotations": pending,
        "timed_out": timed_out,
    })))
}

// ============================================================================
// Annotation SSE
// ============================================================================

/// GET /_dev/annotations/events
pub async fn dev_annotations_sse(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let mut event_rx = state.dev_annotation_tx.subscribe();
    let mut shutdown_rx = state.subscribe_shutdown();

    let stream = async_stream::stream! {
        yield Ok(Event::default().event("connected").data("{}"));

        loop {
            tokio::select! {
                result = event_rx.recv() => {
                    match result {
                        Ok(event) => {
                            let (event_type, data) = match &event {
                                DevAnnotationEvent::Created(ann) => (
                                    "annotation.created",
                                    serde_json::to_string(ann).unwrap_or_default(),
                                ),
                                DevAnnotationEvent::Updated(ann) => (
                                    "annotation.updated",
                                    serde_json::to_string(ann).unwrap_or_default(),
                                ),
                                DevAnnotationEvent::Deleted { id } => (
                                    "annotation.deleted",
                                    serde_json::json!({"id": id}).to_string(),
                                ),
                                DevAnnotationEvent::ThreadMessage(msg) => (
                                    "thread.message",
                                    serde_json::to_string(msg).unwrap_or_default(),
                                ),
                            };
                            yield Ok(Event::default().event(event_type).data(data));
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(lagged = n, "Annotation SSE lagged");
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = shutdown_rx.recv() => break,
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
