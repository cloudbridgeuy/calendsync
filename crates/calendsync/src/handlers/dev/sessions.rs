//! Session list/get/close handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::handlers::error::AppError;
use crate::state::AppState;

use super::get_store;

/// GET /_dev/sessions
#[axum::debug_handler]
pub async fn list_sessions(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let store = get_store(&state)?;
    let sessions: Vec<super::types::DevSession> = store.list_sessions().await?;
    Ok(Json(serde_json::json!({ "sessions": sessions })))
}

/// GET /_dev/sessions/{id}
#[axum::debug_handler]
pub async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, AppError> {
    let store = get_store(&state)?;

    match store.get_session(&id).await? {
        Some(session) => {
            let annotations: Vec<super::types::DevAnnotation> = store.list_by_session(&id).await?;
            Ok(Json(serde_json::json!({
                "session": session,
                "annotations": annotations,
            }))
            .into_response())
        }
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

/// PATCH /_dev/sessions/{id}/close
#[axum::debug_handler]
pub async fn close_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, AppError> {
    let store = get_store(&state)?;

    if store.close_session(&id).await? {
        Ok(StatusCode::OK.into_response())
    } else {
        Ok(StatusCode::NOT_FOUND.into_response())
    }
}
