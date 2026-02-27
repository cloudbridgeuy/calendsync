# Calendar Settings Persistence

Per-user, per-calendar display settings are persisted on the server to eliminate SSR flicker. Previously, settings were stored in localStorage, causing a mismatch between SSR (which always rendered "compact" mode) and client hydration (which read the user's actual preference from localStorage).

## Problem

When a calendar was set to Schedule view, the page would briefly render in Compact mode during SSR, then switch to Schedule on the client. This caused a visible flicker on every page load.

## Solution

Settings are persisted server-side via `SettingsRepository` and injected into the SSR payload (`initial_data.settings`). The frontend reads settings from `initialData.settings` instead of localStorage, and saves changes via a debounced PUT request.

## Data Flow

```
Server                              Client
  │                                    │
  │  get_settings(cal_id, user_id)     │
  ├──────────┐                         │
  │          │  SettingsRepository     │
  │  <───────┘                         │
  │                                    │
  │  initial_data.settings = {...}     │
  ├───────────────────────────────────>│
  │                                    │  useState(initialSettings ?? defaults)
  │                                    │
  │                                    │  [user changes setting]
  │                                    │  setState(newSettings)  // immediate
  │                                    │  setTimeout(500ms) → PUT /api/calendars/{id}/settings
  │  <─────────────────────────────────┤  (fire-and-forget, optimistic)
  │  upsert_settings(...)              │
  │                                    │
```

## Types

```rust
// crates/core/src/calendar/types.rs
pub enum ViewMode { Compact, Schedule }
pub enum EntryStyle { Compact, Filled }
pub struct CalendarSettings {
    pub view_mode: ViewMode,
    pub show_tasks: bool,
    pub entry_style: EntryStyle,
}
```

```typescript
// crates/frontend/src/core/calendar/settings.ts
type ViewMode = "compact" | "schedule"
type EntryStyle = "compact" | "filled"
interface CalendarSettings {
  viewMode: ViewMode
  showTasks: boolean
  entryStyle: EntryStyle
}
```

## Repository Trait

```rust
// crates/core/src/storage/traits.rs
#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get_settings(calendar_id: Uuid, user_id: Uuid) -> Result<Option<CalendarSettings>>;
    async fn upsert_settings(calendar_id: Uuid, user_id: Uuid, settings: &CalendarSettings) -> Result<()>;
}
```

## Storage Backends

| Backend | Status | Notes |
|---------|--------|-------|
| In-memory | Implemented | `HashMap<(Uuid, Uuid), CalendarSettings>` |
| SQLite | Implemented | JSON in `calendar_settings` table, UPSERT with ON CONFLICT |
| DynamoDB | Implemented | `PK=CAL#<cal_id>`, `SK=SETTINGS#<user_id>`, JSON in `settingsJson` attr |

## API Endpoint

`PUT /api/calendars/{id}/settings` — Auth-gated (requires read access to the calendar). No-auth variant returns 200 OK as a no-op.

## SSR Integration

The `calendar_react_ssr` handler calls `get_settings_or_default()` after membership check, then passes settings in `initial_data["settings"]`. Falls back to `CalendarSettings::default()` on missing/error.

## Frontend Hook

`useCalendarSettings` (imperative shell):
- Reads from `initialData.settings` (server-provided)
- State updates are immediate (optimistic UI)
- Saves via debounced PUT (500ms, fire-and-forget)
- `isFirstRender` ref prevents saving initial settings on mount

## Files

| File | Purpose |
|------|---------|
| `crates/core/src/calendar/types.rs` | ViewMode, EntryStyle, CalendarSettings types |
| `crates/core/src/storage/traits.rs` | SettingsRepository trait |
| `crates/calendsync/src/storage/inmemory/repository.rs` | In-memory implementation |
| `crates/calendsync/src/state.rs` | AppState wiring (settings_repo field) |
| `crates/calendsync/src/handlers/settings.rs` | PUT endpoint |
| `crates/calendsync/src/handlers/calendar_react.rs` | SSR integration (get_settings_or_default) |
| `crates/frontend/src/calendsync/hooks/useCalendarSettings.ts` | Hook (reads server, saves via PUT) |
| `crates/frontend/src/calendsync/types.ts` | InitialData.settings field |
| `crates/frontend/src/core/calendar/settings.ts` | Pure settings functions |
