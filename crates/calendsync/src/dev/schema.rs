//! SQL DDL and query constants for the dev annotations SQLite store.
//!
//! Pure module — no runtime logic, only `&str` constants.

// ---------- DDL ----------

pub const PRAGMA_FOREIGN_KEYS: &str = "PRAGMA foreign_keys = ON;";

pub const CREATE_SESSIONS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS sessions (
        id          TEXT PRIMARY KEY,
        url         TEXT NOT NULL,
        status      TEXT NOT NULL DEFAULT 'active',
        created_at  TEXT NOT NULL,
        updated_at  TEXT NOT NULL
    );
"#;

pub const CREATE_ANNOTATIONS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS annotations (
        id              TEXT PRIMARY KEY,
        session_id      TEXT NOT NULL REFERENCES sessions(id),
        url             TEXT NOT NULL,
        element_path    TEXT NOT NULL,
        comment         TEXT NOT NULL,
        tag_name        TEXT NOT NULL,
        text_content    TEXT NOT NULL DEFAULT '',
        component_name  TEXT,
        intent          TEXT NOT NULL DEFAULT 'fix',
        severity        TEXT NOT NULL DEFAULT 'suggestion',
        status          TEXT NOT NULL DEFAULT 'pending',
        selected_text   TEXT,
        nearby_text     TEXT,
        css_classes     TEXT NOT NULL DEFAULT '[]',
        bounding_box    TEXT NOT NULL,
        computed_styles TEXT NOT NULL,
        accessibility   TEXT,
        full_path       TEXT,
        screenshot      TEXT,
        is_fixed        INTEGER NOT NULL DEFAULT 0,
        resolved_at     TEXT,
        resolved_by     TEXT,
        timestamp       TEXT NOT NULL,
        created_at      TEXT NOT NULL,
        updated_at      TEXT NOT NULL
    );
"#;

pub const CREATE_THREAD_MESSAGES_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS thread_messages (
        id              TEXT PRIMARY KEY,
        annotation_id   TEXT NOT NULL REFERENCES annotations(id) ON DELETE CASCADE,
        message         TEXT NOT NULL,
        author          TEXT NOT NULL,
        timestamp       TEXT NOT NULL
    );
"#;

// ---------- Indexes ----------

pub const CREATE_INDEX_SESSIONS_URL: &str =
    "CREATE INDEX IF NOT EXISTS idx_sessions_url ON sessions(url);";

pub const CREATE_INDEX_ANNOTATIONS_SESSION: &str =
    "CREATE INDEX IF NOT EXISTS idx_ann_session ON annotations(session_id);";

pub const CREATE_INDEX_ANNOTATIONS_STATUS: &str =
    "CREATE INDEX IF NOT EXISTS idx_ann_status ON annotations(status);";

pub const CREATE_INDEX_ANNOTATIONS_STATUS_SESSION: &str =
    "CREATE INDEX IF NOT EXISTS idx_ann_status_session ON annotations(status, session_id);";

pub const CREATE_INDEX_THREAD_ANNOTATION: &str =
    "CREATE INDEX IF NOT EXISTS idx_thread_ann ON thread_messages(annotation_id);";

/// Ordered list of all DDL statements for schema initialization.
pub const INIT_SCHEMA: &[&str] = &[
    PRAGMA_FOREIGN_KEYS,
    CREATE_SESSIONS_TABLE,
    CREATE_ANNOTATIONS_TABLE,
    CREATE_THREAD_MESSAGES_TABLE,
    CREATE_INDEX_SESSIONS_URL,
    CREATE_INDEX_ANNOTATIONS_SESSION,
    CREATE_INDEX_ANNOTATIONS_STATUS,
    CREATE_INDEX_ANNOTATIONS_STATUS_SESSION,
    CREATE_INDEX_THREAD_ANNOTATION,
];

// ---------- Session queries ----------

pub const INSERT_SESSION: &str =
    "INSERT INTO sessions (id, url, status, created_at, updated_at) VALUES (?1, ?2, 'active', ?3, ?4);";

pub const SELECT_SESSION_BY_ID: &str =
    "SELECT id, url, status, created_at, updated_at FROM sessions WHERE id = ?1;";

pub const SELECT_SESSION_BY_URL: &str =
    "SELECT id, url, status, created_at, updated_at FROM sessions WHERE url = ?1 AND status = 'active' LIMIT 1;";

pub const SELECT_ALL_SESSIONS: &str =
    "SELECT id, url, status, created_at, updated_at FROM sessions ORDER BY created_at DESC;";

pub const UPDATE_SESSION_STATUS: &str =
    "UPDATE sessions SET status = ?1, updated_at = ?2 WHERE id = ?3;";

// ---------- Annotation queries ----------

pub const INSERT_ANNOTATION: &str = r#"
    INSERT INTO annotations (
        id, session_id, url, element_path, comment, tag_name, text_content,
        component_name, intent, severity, status, selected_text, nearby_text,
        css_classes, bounding_box, computed_styles, accessibility, full_path,
        screenshot, is_fixed, resolved_at, resolved_by, timestamp, created_at, updated_at
    ) VALUES (
        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
        ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25
    );
"#;

pub const SELECT_ANNOTATION_BY_ID: &str = "SELECT * FROM annotations WHERE id = ?1;";

pub const SELECT_ANNOTATIONS_BY_SESSION: &str =
    "SELECT * FROM annotations WHERE session_id = ?1 ORDER BY created_at DESC;";

pub const SELECT_ANNOTATIONS_BY_STATUS: &str =
    "SELECT * FROM annotations WHERE status = ?1 ORDER BY created_at DESC;";

pub const SELECT_ANNOTATIONS_BY_SESSION_AND_STATUS: &str =
    "SELECT * FROM annotations WHERE session_id = ?1 AND status = ?2 ORDER BY created_at DESC;";

pub const SELECT_ALL_ANNOTATIONS: &str = "SELECT * FROM annotations ORDER BY created_at DESC;";

pub const SELECT_PENDING_ANNOTATIONS: &str =
    "SELECT * FROM annotations WHERE status = 'pending' ORDER BY created_at DESC;";

pub const UPDATE_ANNOTATION_STATUS: &str =
    "UPDATE annotations SET status = ?1, resolved_at = ?2, resolved_by = ?3, updated_at = ?4 WHERE id = ?5;";

pub const DELETE_ANNOTATION_BY_ID: &str = "DELETE FROM annotations WHERE id = ?1;";

pub const DELETE_ALL_ANNOTATIONS: &str = "DELETE FROM annotations;";

// ---------- Thread message queries ----------

pub const INSERT_THREAD_MESSAGE: &str =
    "INSERT INTO thread_messages (id, annotation_id, message, author, timestamp) VALUES (?1, ?2, ?3, ?4, ?5);";

pub const SELECT_THREAD_BY_ANNOTATION: &str =
    "SELECT id, annotation_id, message, author, timestamp FROM thread_messages WHERE annotation_id = ?1 ORDER BY timestamp ASC;";
