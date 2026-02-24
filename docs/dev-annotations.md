# Dev Annotation Overlay

The dev annotation overlay lets you click on any UI element in the browser, capture its CSS selector, React component name, computed styles, and a free-text note, then expose that data to Claude Code via MCP tools. This creates a feedback loop where you visually identify issues in the browser and an AI agent can query the structured metadata to understand and fix them.

## Prerequisites

- **mcptools** installed: `cargo xtask install` from the [mcptools](https://github.com/guzmanmonne/mcptools) repo
- **mcptools** registered in Claude Code:
  ```bash
  claude mcp add mcptools -- mcptools mcp stdio
  ```
- **calendsync dev server** running:
  ```bash
  cargo xtask dev server
  ```
- **Browser** open to `http://localhost:3000/calendar/<id>` (create a calendar first, then optionally seed it with `cargo xtask seed <CALENDAR_ID> --session <SESSION_ID>`)

## Creating Annotations (Browser)

1. Click the red **DEV** button in the top-right corner
2. Select **Annotate UI** from the menu
3. Hover over elements — a blue highlight appears with component info
4. Click an element — a note popup opens
5. Type your issue description, then press **Ctrl+Enter** or click **Save**
6. A numbered red marker appears on the annotated element
7. Use **Copy Annotations** to export all annotations as markdown, or **Clear All** to reset

## Querying Annotations (Claude Code)

Once annotations exist, ask Claude Code about them in natural language. The MCP tools handle the rest:

| You say | Tool used |
|---------|-----------|
| "List all UI annotations" | `ui_annotations_list` |
| "Show details for annotation abc-123" | `ui_annotations_get` |
| "Mark annotation abc-123 as resolved — increased font size" | `ui_annotations_resolve` |
| "Clear all annotations" | `ui_annotations_clear` |

If the dev server runs on a non-default address, set the `CALENDSYNC_DEV_URL` environment variable:

```bash
export CALENDSYNC_DEV_URL=http://localhost:8080
```

## REST API Reference

All endpoints are dev-only (require `DEV_MODE` environment variable and a debug build).

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/_dev/annotations` | Create annotation |
| `GET` | `/_dev/annotations` | List all with summary |
| `GET` | `/_dev/annotations/{id}` | Get single annotation |
| `PATCH` | `/_dev/annotations/{id}/resolve` | Mark resolved with summary |
| `DELETE` | `/_dev/annotations/{id}` | Delete one |
| `DELETE` | `/_dev/annotations` | Clear all |

### Example: Create an Annotation

```bash
curl -X POST http://localhost:3000/_dev/annotations \
  -H "Content-Type: application/json" \
  -d '{
    "selector": "div.calendar-header > h1",
    "component_name": "CalendarHeader",
    "tag_name": "h1",
    "text_content": "January 2024",
    "note": "Font size too small on mobile",
    "bounding_box": { "top": 100, "left": 200, "width": 300, "height": 50 },
    "computed_styles": {
      "color": "rgb(0, 0, 0)",
      "background_color": "rgb(255, 255, 255)",
      "font_size": "16px",
      "font_family": "Inter, sans-serif",
      "padding": "8px",
      "margin": "0px",
      "width": "300px",
      "height": "50px",
      "display": "block",
      "position": "relative"
    }
  }'
```
