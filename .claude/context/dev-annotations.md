# Dev Annotation Overlay

UI annotation system for dev mode that captures structured data about UI elements (CSS selectors, React component names, computed styles, bounding boxes, and free-text notes) and exposes them via REST API for MCP integration with Claude Code.

## Architecture

Three layers following Functional Core - Imperative Shell:

### Backend (`crates/calendsync/src/handlers/dev.rs`)

**Pure types and functions:**
- `DevAnnotation`, `BoundingBox`, `ComputedStyles` — serializable annotation data
- `CreateAnnotationRequest` with `into_annotation()` — parse-don't-validate pattern
- `ResolveAnnotationRequest` — marks annotation as addressed
- `find_annotation_index()` — pure lookup by ID
- `count_annotations_summary()` — pure count of resolved/unresolved

**Endpoints (dev-only, behind `DEV_MODE` + `debug_assertions`):**

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/_dev/annotations` | Create annotation |
| `GET` | `/_dev/annotations` | List all with summary |
| `GET` | `/_dev/annotations/{id}` | Get single annotation |
| `PATCH` | `/_dev/annotations/{id}/resolve` | Mark resolved with summary |
| `DELETE` | `/_dev/annotations/{id}` | Delete one |
| `DELETE` | `/_dev/annotations` | Clear all |

**State:** `AppState.dev_annotations: Arc<RwLock<Vec<DevAnnotation>>>` — ephemeral in-memory storage.

### Frontend (`crates/frontend/src/`)

**Pure functions (`core/calendar/annotations.ts`):**
- `generateSelector()` — builds CSS selector path from element metadata
- `extractComputedStyles()` — picks 10 style properties from flat object
- `truncateTextContent()` — trims and truncates with ellipsis
- `formatAnnotationMarkdown()` / `formatAnnotationsMarkdown()` — markdown export
- `buildCreateAnnotationBody()` — assembles request body from element data

**Components (`calendsync/components/`):**
- `DevAnnotationLayer` — connects annotation hook to DevMenu + overlay
- `AnnotationOverlay` — hover highlight, click-to-annotate, markers
- `AnnotationMarker` — numbered circle on annotated elements
- `AnnotationNotePopup` — note input with Ctrl+Enter/Esc shortcuts

**Hook (`calendsync/hooks/useAnnotations.ts`):**
- Fetches annotations on mount, CRUD via `/_dev/annotations`
- `toggle()`, `create()`, `remove()`, `clearAll()`, `copyToClipboard()`

### MCP Tools (mcptools repo)

**Pure functions (`mcptools_core/annotations.rs`):**
- `format_annotation()` — one-line markdown summary
- `format_annotations_list()` — full list with header and instructions
- `format_annotation_detail()` — detailed view with styles and position

**Tools (`mcptools/src/mcp/tools/annotations.rs`):**
- `ui_annotations_list` — list all annotations
- `ui_annotations_get` — get one with full detail
- `ui_annotations_resolve` — mark resolved with summary
- `ui_annotations_clear` — clear all

All tools accept optional `url` param (default: `CALENDSYNC_DEV_URL` env or `http://localhost:3000`).

## Usage

1. Run dev server: `cargo xtask dev server --seed`
2. Open browser, click DEV menu, select "Annotate UI"
3. Hover elements to see highlight + component info
4. Click element, type note, save
5. Claude Code queries via MCP: `ui_annotations_list`, then `ui_annotations_get <id>`
6. After fixing: `ui_annotations_resolve <id> "description of fix"`

## Testing

```bash
# Backend
cargo test -p calendsync -- annotation

# Frontend
cd crates/frontend && bun test

# MCP (mcptools repo)
cargo test -p mcptools_core -- annotation
```
