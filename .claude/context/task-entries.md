# Task Entries

This document describes how task entries work in the calendsync calendar.

## Overview

Tasks are a special type of calendar entry with a checkbox that can be toggled between complete/incomplete states. Unlike regular events, tasks persist their completion state.

## Entry Type System

Calendar entries use the `EntryKind` enum to distinguish between types:

```rust
pub enum EntryKind {
    MultiDay { start: NaiveDate, end: NaiveDate },
    AllDay,
    Timed { start: NaiveTime, end: NaiveTime },
    Task { completed: bool },
}
```

## Data Format Transformation

The backend uses Rust's `CalendarEntry` with nested enums, but the frontend expects a flattened `ServerEntry` format with boolean flags.

### Backend Format (CalendarEntry)
```json
{
  "kind": { "Task": { "completed": false } },
  "title": "Buy groceries"
}
```

### Frontend Format (ServerEntry)
```json
{
  "kind": "task",
  "isTask": true,
  "completed": false,
  "title": "Buy groceries"
}
```

### Transformation Function

The `entry_to_server_entry()` function in `handlers/calendar_react.rs` handles this transformation. It's used by:

- **SSR**: Initial page load transforms entries for React hydration
- **API responses**: `create_entry`, `update_entry`, `toggle_entry` return ServerEntry format
- **SSE events**: Real-time updates are transformed before broadcasting

## Toggle API

Tasks can be toggled via:

```
PATCH /api/entries/{id}/toggle
```

This endpoint:
1. Flips the `completed` boolean
2. Publishes an SSE `entry_updated` event
3. Returns the updated entry in ServerEntry format

## Frontend Implementation

### EntryTile Component

Task entries render with an `<input type="checkbox">` that:
- Shows checked/unchecked based on `entry.completed`
- Calls `onToggle` handler on click
- Stops event propagation to prevent opening the edit modal

### Optimistic Updates

When the checkbox is clicked:
1. UI immediately reflects the toggled state (optimistic update)
2. API call is made in background
3. On failure, the state reverts to original

### Key Files

| File | Purpose |
|------|---------|
| `crates/calendsync/src/handlers/entries.rs` | Toggle endpoint, API responses |
| `crates/calendsync/src/handlers/events.rs` | SSE event serialization |
| `crates/calendsync/src/handlers/calendar_react.rs` | `entry_to_server_entry()` transformation |
| `crates/frontend/src/calendar-react/components/EntryTile.tsx` | Task checkbox UI |
| `crates/frontend/src/calendar-react/hooks/useEntryApi.ts` | `toggleEntry()` API call |
| `crates/frontend/src/calendar-react/components/Calendar.tsx` | Toggle handler with optimistic update |
