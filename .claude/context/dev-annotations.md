# Dev Annotation System

UI annotation system for dev mode that captures structured feedback about UI elements and persists it in SQLite. Integrates with Claude Code via MCP tools for collaborative UI review.

## Setup

For the full human-facing guide, see [`docs/dev-annotations.md`](../../docs/dev-annotations.md).

**Start the dev server with annotations enabled:**

```bash
cargo xtask dev server --annotations
```

**Register mcptools in Claude Code:**

```bash
claude mcp add mcptools -- mcptools mcp stdio
```

**Environment variable:** Set `CALENDSYNC_DEV_URL` if the dev server runs on a non-default address (default: `http://localhost:3000`).

## Feature Gate

The `dev-annotations` feature is opt-in. It gates:

- Rust dependencies: `rusqlite`, `tokio-rusqlite`
- Backend modules: `src/dev/` (store), `src/handlers/dev/annotations.rs`, `src/handlers/dev/sessions.rs`
- Route registration in `app.rs`
- State fields in `AppState`

Without the feature, hot-reload endpoints still work. Only annotation/session endpoints require it.

## Architecture

Three layers following Functional Core - Imperative Shell:

### Backend

**Persistence (`crates/calendsync/src/dev/`):**
- `schema.rs` — Pure SQL DDL constants (3 tables: sessions, annotations, thread_messages)
- `store.rs` — `DevAnnotationStore` wrapping `tokio_rusqlite::Connection`

**Domain types (`crates/calendsync/src/handlers/dev/types.rs`):**
- `AnnotationIntent` — fix, change, question, approve
- `AnnotationSeverity` — blocking, important, suggestion
- `AnnotationStatus` — pending, acknowledged, resolved, dismissed
- `DevAnnotation`, `ThreadMessage`, `DevSession` — serializable domain structs
- Pure functions: `count_annotations_summary()`, `Display` impls

**Handlers (`crates/calendsync/src/handlers/dev/`):**
- `annotations.rs` — CRUD, status transitions, threading, SSE
- `sessions.rs` — Session list/get/close
- `hot_reload.rs` — SSR reload + build error reporting (always available)

**Endpoints (behind `DEV_MODE` + `dev-annotations` feature):**

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/_dev/annotations` | Create annotation (auto-creates session) |
| `GET` | `/_dev/annotations` | List all with summary |
| `GET` | `/_dev/annotations/pending` | List pending only |
| `GET` | `/_dev/annotations/watch` | Long-poll for changes |
| `GET` | `/_dev/annotations/events` | SSE stream |
| `GET` | `/_dev/annotations/{id}` | Get single with thread |
| `DELETE` | `/_dev/annotations/{id}` | Delete one |
| `DELETE` | `/_dev/annotations` | Clear all |
| `PATCH` | `/_dev/annotations/{id}/acknowledge` | Mark acknowledged |
| `PATCH` | `/_dev/annotations/{id}/resolve` | Mark resolved |
| `PATCH` | `/_dev/annotations/{id}/dismiss` | Mark dismissed |
| `POST` | `/_dev/annotations/{id}/thread` | Add thread message |
| `GET` | `/_dev/sessions` | List sessions |
| `GET` | `/_dev/sessions/{id}` | Get session |
| `PATCH` | `/_dev/sessions/{id}/close` | Close session |

**State:** `AppState.dev_store: Option<Arc<DevAnnotationStore>>` + `dev_annotation_tx: broadcast::Sender` for SSE.

**Storage:** SQLite at `data/dev-annotations.db` (configurable via `DEV_ANNOTATIONS_DB_PATH`). Persists across server restarts. Delete the DB file to clear all annotations.

### Frontend (`crates/frontend/src/`)

**Pure functions (`core/calendar/annotations.ts`):**
- `generateSelector()` — builds CSS selector path from element metadata
- `extractComputedStyles()` — picks style properties from flat object
- `truncateTextContent()` — trims and truncates with ellipsis
- `statusColor()`, `intentIcon()` — display helpers
- `groupByStatus()`, `formatAnnotationsMarkdown()` — grouping and export
- `buildCreateAnnotationBody()` — assembles request body from element data

**Feature detection:** Backend sends `annotationsEnabled: cfg!(feature = "dev-annotations")` in SSR initial data. `DevAnnotationLayer` only renders when this flag is true, preventing 404s when the feature isn't compiled in.

**Components (`calendsync/components/`):**
- `DevAnnotationLayer` — client-only wrapper connecting hook to overlay (gated on `annotationsEnabled`)
- `AnnotationOverlay` — hover highlight, click-to-annotate, renders markers
- `AnnotationMarker` — color-coded numbered circles on annotated elements
- `AnnotationNotePopup` — note input with intent/severity dropdowns
- `AnnotationDetailPopup` — detail view with metadata, thread, status actions

**Hook (`calendsync/hooks/useAnnotations.ts`):**
- SSE real-time sync with exponential backoff reconnect
- CRUD: `create()`, `remove()`, `clearAll()`
- Status: `acknowledge()`, `resolve()`, `dismiss()`
- Threading: `reply()`
- Clipboard: `copyToClipboard()`

### MCP Tools (mcptools repo)

- `ui_annotations_list` — list all annotations
- `ui_annotations_get` — get one with full detail
- `ui_annotations_resolve` — mark resolved with summary
- `ui_annotations_clear` — clear all

All tools accept optional `url` param (default: `CALENDSYNC_DEV_URL` env or `http://localhost:3000`).

## Usage

1. Run dev server: `cargo xtask dev server --annotations`
2. Open browser, click DEV menu, select "Annotate UI"
3. Hover elements to see highlight + component info
4. Click element, fill note with intent/severity, save
5. Claude Code queries via MCP: `ui_annotations_list`, then `ui_annotations_get <id>`
6. After fixing: `ui_annotations_resolve <id> "description of fix"`
7. Annotations persist across server restarts in SQLite

## For Claude

Always use the MCP tools (`ui_annotations_list`, `ui_annotations_get`, `ui_annotations_resolve`, `ui_annotations_clear`) to query and manage annotations. Never query the SQLite database (`data/dev-annotations.db`) directly. The dev server must be running with `cargo xtask dev server --annotations`.

## Testing

```bash
# Frontend pure functions
cd crates/frontend && bun test

# MCP (mcptools repo)
cargo test -p mcptools_core -- annotation
```
