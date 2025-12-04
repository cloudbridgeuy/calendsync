# SSE Event Publishing

This document describes the Server-Sent Events (SSE) implementation for real-time calendar updates.

## Overview

Calendar entry operations (create, update, delete, toggle) publish SSE events to connected clients, enabling real-time UI updates without polling.

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Entry Handler  │────▶│   AppState      │────▶│  SSE Stream     │
│  (create/update/│     │  publish_event()│     │  (events.rs)    │
│   delete/toggle)│     │                 │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                               │
                               ▼
                        ┌─────────────────┐
                        │  Event History  │
                        │  (VecDeque)     │
                        │  max 1000 events│
                        └─────────────────┘
```

## Event Types

Defined in `calendsync_core::calendar::CalendarEvent`:

```rust
pub enum CalendarEvent {
    EntryAdded { entry: CalendarEntry },
    EntryUpdated { entry: CalendarEntry },
    EntryDeleted { entry_id: Uuid, date: NaiveDate },
}
```

## Server Implementation

### Publishing Events

Entry handlers call `state.publish_event()` after modifying the store:

```rust
// In create_entry handler
state.entries.write().insert(entry.id, entry.clone());
state.publish_event(entry.calendar_id, CalendarEvent::entry_added(entry.clone()));

// In update_entry handler
let updated_entry = {
    let mut entries = state.entries.write();
    let entry = entries.get_mut(&id).ok_or(...)?;
    payload.apply_to(entry);
    entry.clone()
}; // Lock released before publishing
state.publish_event(updated_entry.calendar_id, CalendarEvent::entry_updated(updated_entry.clone()));

// In delete_entry handler
if let Some(entry) = state.entries.write().remove(&id) {
    state.publish_event(
        entry.calendar_id,
        CalendarEvent::entry_deleted(entry.id, entry.date),
    );
}
```

### Event Storage

Events are stored with incrementing IDs for reconnection catch-up:

```rust
pub struct StoredEvent {
    pub id: u64,          // Unique incrementing ID
    pub calendar_id: Uuid, // Filter for calendar-specific streams
    pub event: CalendarEvent,
}
```

The `publish_event()` method:
1. Generates unique event ID via atomic counter
2. Stores event in `event_history` (VecDeque)
3. Trims history to max 1000 events

### SSE Stream

Located in `handlers/events.rs`. The stream:

1. **Initial catch-up**: Sends missed events since `last_event_id` query param
2. **Active polling**: Polls for new events every 100ms
3. **Event format**: SSE with event type, ID, and JSON data

```
GET /api/events?calendar_id=...&last_event_id=0

event: entry_added
id: 1
data: {"entry": {...}}

event: entry_deleted
id: 2
data: {"entry_id": "...", "date": "2025-01-01"}
```

## Client Implementation

### useSSE Hook

Located in `crates/frontend/src/calendar-react/hooks/useSSE.ts`:

```typescript
// Connect to SSE stream
const eventSource = new EventSource(
    `/api/events?calendar_id=${calendarId}&last_event_id=${lastEventId}`
);

// Handle events
eventSource.addEventListener('entry_added', (e) => {
    const data = JSON.parse(e.data);
    // Add entry to cache
});

eventSource.addEventListener('entry_deleted', (e) => {
    const data = JSON.parse(e.data);
    // Remove entry from cache
});
```

### Reconnection

On disconnect:
1. Store the last received event ID
2. Reconnect with `last_event_id` parameter
3. Server sends all missed events

## Files

| File | Description |
|------|-------------|
| `crates/calendsync/src/state.rs` | `publish_event()`, `StoredEvent`, event history |
| `crates/calendsync/src/handlers/events.rs` | SSE stream endpoint with polling |
| `crates/calendsync/src/handlers/entries.rs` | Entry handlers that publish events |
| `crates/calendsync_core/src/calendar/types.rs` | `CalendarEvent` enum |
| `crates/frontend/src/calendar-react/hooks/useSSE.ts` | Client SSE hook |

## Configuration

- **Poll interval**: 100ms (in `events.rs`)
- **Max event history**: 1000 events (in `state.rs`)
- **Max session duration**: 1 hour (in `events.rs`)
- **Keep-alive**: Handled by axum's `Sse::keep_alive()`

## Testing

Start the server and use curl to test:

```bash
# Watch SSE stream
curl -N "http://localhost:3000/api/events?calendar_id=00000000-0000-0000-0000-000000000001&last_event_id=0"

# Create entry (in another terminal)
curl -X POST "http://localhost:3000/api/entries" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "calendar_id=00000000-0000-0000-0000-000000000001&title=Test&date=2025-01-01&kind=all_day"
```

You should see the `entry_added` event in the SSE stream.
