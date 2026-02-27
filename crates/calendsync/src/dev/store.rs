//! Async SQLite-backed store for dev annotations.
//!
//! Imperative Shell — wraps `tokio_rusqlite::Connection` and delegates
//! to the pure SQL constants in [`super::schema`].

use rusqlite::OptionalExtension;

use crate::handlers::dev::types::{
    AnnotationIntent, AnnotationSeverity, AnnotationStatus, DevAnnotation, DevSession,
    SessionStatus, ThreadAuthor, ThreadMessage,
};

use super::schema;

/// Async dev annotation store backed by SQLite.
pub struct DevAnnotationStore {
    conn: tokio_rusqlite::Connection,
}

impl DevAnnotationStore {
    /// Open (or create) a SQLite database at `db_path` and initialize the schema.
    pub async fn new(db_path: &str) -> Result<Self, anyhow::Error> {
        let conn = tokio_rusqlite::Connection::open(db_path).await?;

        conn.call(|conn| {
            for ddl in schema::INIT_SCHEMA {
                conn.execute_batch(ddl)?;
            }
            Ok(())
        })
        .await?;

        Ok(Self { conn })
    }

    // ========================================================================
    // Sessions
    // ========================================================================

    /// Find an active session for `url`, or create one.
    pub async fn find_or_create_session(&self, url: &str) -> Result<DevSession, anyhow::Error> {
        let url_owned = url.to_owned();
        self.conn
            .call(move |conn| {
                let existing: Option<DevSession> = conn
                    .prepare(schema::SELECT_SESSION_BY_URL)?
                    .query_row(rusqlite::params![&url_owned], row_to_session)
                    .optional()?;

                if let Some(session) = existing {
                    return Ok(session);
                }

                let now = chrono::Utc::now().to_rfc3339();
                let id = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    schema::INSERT_SESSION,
                    rusqlite::params![&id, &url_owned, &now, &now],
                )?;

                Ok(DevSession {
                    id,
                    url: url_owned,
                    status: SessionStatus::Active,
                    created_at: now.clone(),
                    updated_at: now,
                })
            })
            .await
            .map_err(Into::into)
    }

    /// Get a session by ID.
    pub async fn get_session(&self, id: &str) -> Result<Option<DevSession>, anyhow::Error> {
        let id_owned = id.to_owned();
        self.conn
            .call(move |conn| {
                conn.prepare(schema::SELECT_SESSION_BY_ID)?
                    .query_row(rusqlite::params![&id_owned], row_to_session)
                    .optional()
                    .map_err(Into::into)
            })
            .await
            .map_err(Into::into)
    }

    /// List all sessions.
    pub async fn list_sessions(&self) -> Result<Vec<DevSession>, anyhow::Error> {
        self.conn
            .call(|conn| {
                let mut stmt = conn.prepare(schema::SELECT_ALL_SESSIONS)?;
                let sessions = stmt
                    .query_map([], row_to_session)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(sessions)
            })
            .await
            .map_err(Into::into)
    }

    /// Close a session.
    pub async fn close_session(&self, id: &str) -> Result<bool, anyhow::Error> {
        let id_owned = id.to_owned();
        self.conn
            .call(move |conn| {
                let now = chrono::Utc::now().to_rfc3339();
                let rows = conn.execute(
                    schema::UPDATE_SESSION_STATUS,
                    rusqlite::params!["closed", &now, &id_owned],
                )?;
                Ok(rows > 0)
            })
            .await
            .map_err(Into::into)
    }

    // ========================================================================
    // Annotations
    // ========================================================================

    /// Insert a new annotation.
    pub async fn create_annotation(&self, ann: &DevAnnotation) -> Result<(), anyhow::Error> {
        let ann = ann.clone();
        self.conn
            .call(move |conn| {
                let now = chrono::Utc::now().to_rfc3339();
                let css_classes_json = serde_json::to_string(&ann.css_classes).unwrap_or_default();
                let bbox_json = serde_json::to_string(&ann.bounding_box).unwrap_or_default();
                let styles_json = serde_json::to_string(&ann.computed_styles).unwrap_or_default();
                let a11y_json = ann
                    .accessibility
                    .as_ref()
                    .and_then(|a| serde_json::to_string(a).ok());

                conn.execute(
                    schema::INSERT_ANNOTATION,
                    rusqlite::params![
                        &ann.id,
                        &ann.session_id,
                        &ann.url,
                        &ann.element_path,
                        &ann.comment,
                        &ann.tag_name,
                        &ann.text_content,
                        &ann.component_name,
                        serialize_intent(ann.intent),
                        serialize_severity(ann.severity),
                        serialize_status(ann.status),
                        &ann.selected_text,
                        &ann.nearby_text,
                        &css_classes_json,
                        &bbox_json,
                        &styles_json,
                        &a11y_json,
                        &ann.full_path,
                        &ann.screenshot,
                        ann.is_fixed as i32,
                        &ann.resolved_at,
                        &ann.resolved_by,
                        &ann.timestamp,
                        &now,
                        &now,
                    ],
                )?;
                Ok(())
            })
            .await
            .map_err(Into::into)
    }

    /// Get a single annotation by ID, including its thread messages.
    pub async fn get_annotation(&self, id: &str) -> Result<Option<DevAnnotation>, anyhow::Error> {
        let id_owned = id.to_owned();
        self.conn
            .call(move |conn| {
                let mut ann: DevAnnotation = match conn
                    .prepare(schema::SELECT_ANNOTATION_BY_ID)?
                    .query_row(rusqlite::params![&id_owned], row_to_annotation)
                    .optional()?
                {
                    Some(a) => a,
                    None => return Ok(None),
                };

                // Load thread messages
                let mut stmt = conn.prepare(schema::SELECT_THREAD_BY_ANNOTATION)?;
                ann.thread = stmt
                    .query_map(rusqlite::params![&id_owned], row_to_thread_message)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;

                Ok(Some(ann))
            })
            .await
            .map_err(Into::into)
    }

    /// List annotations filtered by session and/or status.
    pub async fn list_filtered(
        &self,
        session_id: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<DevAnnotation>, anyhow::Error> {
        let session_id = session_id.map(ToOwned::to_owned);
        let status = status.map(ToOwned::to_owned);

        self.conn
            .call(move |conn| {
                let (sql, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) =
                    match (&session_id, &status) {
                        (Some(sid), Some(st)) => (
                            schema::SELECT_ANNOTATIONS_BY_SESSION_AND_STATUS,
                            vec![Box::new(sid.clone()), Box::new(st.clone())],
                        ),
                        (Some(sid), None) => (
                            schema::SELECT_ANNOTATIONS_BY_SESSION,
                            vec![Box::new(sid.clone())],
                        ),
                        (None, Some(st)) => (
                            schema::SELECT_ANNOTATIONS_BY_STATUS,
                            vec![Box::new(st.clone())],
                        ),
                        (None, None) => (schema::SELECT_ALL_ANNOTATIONS, vec![]),
                    };

                let mut stmt = conn.prepare(sql)?;
                let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                    params.iter().map(|p| p.as_ref()).collect();
                let annotations = stmt
                    .query_map(param_refs.as_slice(), row_to_annotation)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(annotations)
            })
            .await
            .map_err(Into::into)
    }

    /// List pending annotations.
    pub async fn list_pending(&self) -> Result<Vec<DevAnnotation>, anyhow::Error> {
        self.conn
            .call(|conn| {
                let mut stmt = conn.prepare(schema::SELECT_PENDING_ANNOTATIONS)?;
                let annotations = stmt
                    .query_map([], row_to_annotation)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(annotations)
            })
            .await
            .map_err(Into::into)
    }

    /// Update an annotation's status.
    pub async fn update_status(
        &self,
        id: &str,
        status: AnnotationStatus,
        resolved_by: Option<&str>,
    ) -> Result<bool, anyhow::Error> {
        let id_owned = id.to_owned();
        let status_str = serialize_status(status).to_owned();
        let resolved_by = resolved_by.map(ToOwned::to_owned);
        let resolved_at = if matches!(status, AnnotationStatus::Resolved) {
            Some(chrono::Utc::now().to_rfc3339())
        } else {
            None
        };

        self.conn
            .call(move |conn| {
                let now = chrono::Utc::now().to_rfc3339();
                let rows = conn.execute(
                    schema::UPDATE_ANNOTATION_STATUS,
                    rusqlite::params![&status_str, &resolved_at, &resolved_by, &now, &id_owned],
                )?;
                Ok(rows > 0)
            })
            .await
            .map_err(Into::into)
    }

    /// Delete an annotation by ID.
    pub async fn delete_annotation(&self, id: &str) -> Result<bool, anyhow::Error> {
        let id_owned = id.to_owned();
        self.conn
            .call(move |conn| {
                let rows = conn.execute(
                    schema::DELETE_ANNOTATION_BY_ID,
                    rusqlite::params![&id_owned],
                )?;
                Ok(rows > 0)
            })
            .await
            .map_err(Into::into)
    }

    /// Delete all annotations.
    pub async fn clear_all(&self) -> Result<usize, anyhow::Error> {
        self.conn
            .call(|conn| {
                let rows = conn.execute(schema::DELETE_ALL_ANNOTATIONS, [])?;
                Ok(rows)
            })
            .await
            .map_err(Into::into)
    }

    // ========================================================================
    // Thread messages
    // ========================================================================

    /// Add a thread message to an annotation.
    pub async fn add_thread_message(&self, msg: &ThreadMessage) -> Result<(), anyhow::Error> {
        let msg = msg.clone();
        self.conn
            .call(move |conn| {
                conn.execute(
                    schema::INSERT_THREAD_MESSAGE,
                    rusqlite::params![
                        &msg.id,
                        &msg.annotation_id,
                        &msg.message,
                        serialize_author(msg.author),
                        &msg.timestamp,
                    ],
                )?;
                Ok(())
            })
            .await
            .map_err(Into::into)
    }

    /// List all annotations for a session (without threads).
    pub async fn list_by_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<DevAnnotation>, anyhow::Error> {
        let sid = session_id.to_owned();
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(schema::SELECT_ANNOTATIONS_BY_SESSION)?;
                let annotations = stmt
                    .query_map(rusqlite::params![&sid], row_to_annotation)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(annotations)
            })
            .await
            .map_err(Into::into)
    }
}

// ============================================================================
// Row mappers (stateless helpers)
// ============================================================================

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<DevSession> {
    Ok(DevSession {
        id: row.get("id")?,
        url: row.get("url")?,
        status: parse_session_status(&row.get::<_, String>("status")?),
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_annotation(row: &rusqlite::Row<'_>) -> rusqlite::Result<DevAnnotation> {
    let css_json: String = row.get("css_classes")?;
    let bbox_json: String = row.get("bounding_box")?;
    let styles_json: String = row.get("computed_styles")?;
    let access_json: Option<String> = row.get("accessibility")?;

    Ok(DevAnnotation {
        id: row.get("id")?,
        session_id: row.get("session_id")?,
        url: row.get("url")?,
        element_path: row.get("element_path")?,
        comment: row.get("comment")?,
        tag_name: row.get("tag_name")?,
        text_content: row.get("text_content")?,
        component_name: row.get("component_name")?,
        intent: parse_intent(&row.get::<_, String>("intent")?),
        severity: parse_severity(&row.get::<_, String>("severity")?),
        status: parse_status(&row.get::<_, String>("status")?),
        selected_text: row.get("selected_text")?,
        nearby_text: row.get("nearby_text")?,
        css_classes: serde_json::from_str(&css_json).unwrap_or_default(),
        bounding_box: serde_json::from_str(&bbox_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        computed_styles: serde_json::from_str(&styles_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        accessibility: access_json.and_then(|j| serde_json::from_str(&j).ok()),
        full_path: row.get("full_path")?,
        screenshot: row.get("screenshot")?,
        is_fixed: row.get::<_, i32>("is_fixed")? != 0,
        resolved_at: row.get("resolved_at")?,
        resolved_by: row.get("resolved_by")?,
        timestamp: row.get("timestamp")?,
        thread: Vec::new(), // Loaded separately via get_annotation()
    })
}

fn row_to_thread_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<ThreadMessage> {
    Ok(ThreadMessage {
        id: row.get("id")?,
        annotation_id: row.get("annotation_id")?,
        message: row.get("message")?,
        author: parse_author(&row.get::<_, String>("author")?),
        timestamp: row.get("timestamp")?,
    })
}

// ============================================================================
// Enum serialization helpers
// ============================================================================

fn parse_session_status(s: &str) -> SessionStatus {
    match s {
        "closed" => SessionStatus::Closed,
        _ => SessionStatus::Active,
    }
}

fn parse_intent(s: &str) -> AnnotationIntent {
    match s {
        "change" => AnnotationIntent::Change,
        "question" => AnnotationIntent::Question,
        "approve" => AnnotationIntent::Approve,
        _ => AnnotationIntent::Fix,
    }
}

fn parse_severity(s: &str) -> AnnotationSeverity {
    match s {
        "blocking" => AnnotationSeverity::Blocking,
        "important" => AnnotationSeverity::Important,
        _ => AnnotationSeverity::Suggestion,
    }
}

fn parse_status(s: &str) -> AnnotationStatus {
    match s {
        "acknowledged" => AnnotationStatus::Acknowledged,
        "resolved" => AnnotationStatus::Resolved,
        "dismissed" => AnnotationStatus::Dismissed,
        _ => AnnotationStatus::Pending,
    }
}

fn parse_author(s: &str) -> ThreadAuthor {
    match s {
        "agent" => ThreadAuthor::Agent,
        _ => ThreadAuthor::Human,
    }
}

fn serialize_intent(i: AnnotationIntent) -> &'static str {
    match i {
        AnnotationIntent::Fix => "fix",
        AnnotationIntent::Change => "change",
        AnnotationIntent::Question => "question",
        AnnotationIntent::Approve => "approve",
    }
}

fn serialize_severity(s: AnnotationSeverity) -> &'static str {
    match s {
        AnnotationSeverity::Blocking => "blocking",
        AnnotationSeverity::Important => "important",
        AnnotationSeverity::Suggestion => "suggestion",
    }
}

fn serialize_status(s: AnnotationStatus) -> &'static str {
    match s {
        AnnotationStatus::Pending => "pending",
        AnnotationStatus::Acknowledged => "acknowledged",
        AnnotationStatus::Resolved => "resolved",
        AnnotationStatus::Dismissed => "dismissed",
    }
}

fn serialize_author(a: ThreadAuthor) -> &'static str {
    match a {
        ThreadAuthor::Human => "human",
        ThreadAuthor::Agent => "agent",
    }
}
