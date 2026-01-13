# SSE Hooks

## Overview

Unified SSE implementation supporting both web (EventSource) and Tauri (IPC) platforms.

The SSE hooks follow the **Functional Core - Imperative Shell** pattern, with pure connection logic extracted to `@core/sse` and platform-specific I/O operations in the hooks.

## Web Implementation (useWebSse)

### Features

- Native EventSource for browser SSE
- Exponential backoff reconnection (3s, 6s, 12s, 24s, capped)
- URL encoding for calendar IDs and event IDs
- Error callbacks for event handling failures
- Connection state tracking
- Automatic sync status updates via Dexie

### API

```typescript
useWebSse({
  calendarId: string
  enabled?: boolean
  onEntryAdded?: (entry: ServerEntry, date: string) => void
  onEntryUpdated?: (entry: ServerEntry, date: string) => void
  onEntryDeleted?: (entryId: string, date: string) => void
  onConnectionChange?: (state: SseConnectionState) => void
  onError?: (error: Error, context: string) => void
})
```

### Error Contexts

- `"parse_entry_added"` - Failed to parse entry_added event JSON
- `"parse_entry_updated"` - Failed to parse entry_updated event JSON
- `"parse_entry_deleted"` - Failed to parse entry_deleted event JSON
- `"handle_entry_added"` - Failed to handle entry_added event (Dexie error)
- `"handle_entry_updated"` - Failed to handle entry_updated event (Dexie error)
- `"handle_entry_deleted"` - Failed to handle entry_deleted event (Dexie error)
- `"max_reconnect_attempts"` - Max reconnection attempts reached (5 attempts)

### Data Flow

1. SSE event received from server via EventSource
2. Event parsed using pure function `parseEventData` from `@core/sse/connection`
3. Entry updated in Dexie with `syncStatus: "synced"`
4. `last_event_id` saved to `sync_state` table for reconnection
5. `useLiveQuery` in `useOfflineCalendar` reactively updates UI

### Exponential Backoff

The hook uses `calculateReconnectDelay` from `@core/sse/connection` for exponential backoff:

- Attempt 1: 3s (1.5s × 2^1)
- Attempt 2: 6s (1.5s × 2^2)
- Attempt 3: 12s (1.5s × 2^3)
- Attempt 4: 24s (1.5s × 2^4, capped at max exponent)
- Attempt 5+: 24s (capped)

After 5 failed attempts, the connection enters error state and stops retrying.

## Tauri Implementation (useTauriSse)

### Features

- Tauri event system for IPC SSE
- Session cookie authentication via Rust backend
- Error callbacks for event handling failures
- Connection state tracking
- Automatic sync status updates via Dexie

### API

```typescript
useTauriSse({
  calendarId: string
  enabled?: boolean
  onEntryAdded?: (entry: ServerEntry, date: string) => void
  onEntryUpdated?: (entry: ServerEntry, date: string) => void
  onEntryDeleted?: (entryId: string, date: string) => void
  onConnectionChange?: (state: SseConnectionState) => void
  onError?: (error: Error, context: string) => void
})
```

### Error Contexts

Same parsing and handler contexts as useWebSse, plus:

- `"start_sse"` - Failed to start SSE connection via Tauri command
- `"reconnect_sse"` - Failed to reconnect SSE via Tauri command
- `"disconnect_sse"` - Failed to disconnect SSE via Tauri command

### Data Flow

1. Frontend calls `start_sse` Tauri command with `calendar_id`
2. Rust backend connects to `/api/events` with session cookie
3. Rust backend parses SSE messages and emits Tauri events
4. Hook receives events via `listen()` and updates Dexie
5. `useLiveQuery` in `useOfflineCalendar` reactively updates UI

### Rust Backend

The Tauri implementation uses:

- `crates/src-tauri/src/sse.rs` - Pure SSE parsing logic (`parse_sse_message`)
- HTTP client with session authentication
- Event emission via `app.emit()` for IPC

## Shared Components

### useConnectionManager

Extracts common connection state management pattern:

```typescript
const { connectionState, updateConnectionState } = useConnectionManager({
  onConnectionChange: config.onConnectionChange,
})
```

**Features**:
- Manages `SseConnectionState` ("disconnected" | "connecting" | "connected" | "error")
- Stores callbacks in refs to avoid triggering reconnections
- Provides memoized `updateConnectionState` function

**Usage**: Both `useWebSse` and `useTauriSse` use this hook for consistent state management.

### useDexieHandlers

Shared database update logic using pure functions from `@core/sse/sync`:

```typescript
const { handleEntryAdded, handleEntryUpdated, handleEntryDeleted } = useDexieHandlers()
```

**Features**:
- Uses `determineSyncAction` to decide whether to update or confirm creation
- Uses `determineUpdateSyncAction` to decide whether to update or skip
- Handles optimistic UI updates (pending operations)
- Updates `last_event_id` in sync_state table

**Pure Logic**: Decision logic is in `@core/sse/sync.ts`:
- `determineSyncAction(existing)` - Returns "create" | "confirm_create" | "skip"
- `determineUpdateSyncAction(existing)` - Returns "update" | "skip"

## Unified Hook (useSseUnified)

**File**: `crates/frontend/src/calendsync/hooks/useSseUnified.ts`

Automatically selects the correct implementation based on platform:

```typescript
import { useSseUnified } from "@calendsync/hooks"

const { connectionState } = useSseUnified({
  calendarId: "cal_123",
  onConnectionChange: (state) => console.log(state),
})
```

**Implementation**:
- Uses `isTauri` from `@core/transport` to detect platform
- Returns `useTauriSse` on Tauri, `useWebSse` on web
- Provides consistent API across platforms

## Functional Core - Imperative Shell

### Functional Core (Pure Functions)

**Location**: `crates/frontend/src/core/sse/`

Pure functions with no side effects:

- `buildSseUrl(baseUrl, calendarId, lastEventId?)` - URL construction with encoding
- `parseEventData<T>(data)` - Safe JSON parsing with null return on error
- `calculateReconnectDelay(attempts)` - Exponential backoff calculation
- `shouldReconnect(attempts)` - Boolean check for max attempts
- `determineSyncAction(existing?)` - Sync decision logic
- `determineUpdateSyncAction(existing?)` - Update decision logic

**Testing**: All pure functions have unit tests in `@core/sse/__tests__/`

### Imperative Shell (I/O Operations)

**Location**: `crates/frontend/src/calendsync/hooks/`

Hooks that perform I/O:

- `useWebSse` - EventSource creation, network requests
- `useTauriSse` - Tauri IPC, event listeners
- `useConnectionManager` - React state management
- `useDexieHandlers` - Dexie database updates

**Testing**: No unit tests required per project testing policy (imperative shell)

## Migration Guide

### Adding Error Handling

Both hooks now support optional error callbacks:

```typescript
// Before
useWebSse({ calendarId })

// After - with error handling
useWebSse({
  calendarId,
  onError: (error, context) => {
    if (context === "max_reconnect_attempts") {
      showNotification("Lost connection to server")
    }
    console.error(`SSE error [${context}]:`, error)
  },
})
```

### URL Encoding

The hooks now use `buildSseUrl` for proper URL encoding:

```typescript
// Before (manual, no encoding)
let url = `${baseUrl}/api/events?calendar_id=${calendarId}`

// After (uses buildSseUrl with encodeURIComponent)
const url = buildSseUrl(baseUrl, calendarId, lastEventId)
```

This prevents issues with special characters in calendar IDs.

## Troubleshooting

### Connection State Stuck in "connecting"

**Cause**: EventSource failed to open but didn't trigger `onerror`

**Solution**: Check browser console for CORS errors or network issues

### Events Not Updating UI

**Cause**: Dexie handler failed silently

**Solution**: Add `onError` callback to see handler errors:

```typescript
useWebSse({
  calendarId,
  onError: (error, context) => {
    if (context.startsWith("handle_")) {
      console.error("Failed to update Dexie:", error)
    }
  },
})
```

### Max Reconnect Attempts Reached

**Cause**: Server unreachable or repeatedly returning errors

**Solution**:
1. Check server is running and accessible
2. Verify session authentication is valid
3. Increase `MAX_RECONNECT_ATTEMPTS` if needed (currently 5)

### Tauri SSE Not Connecting

**Cause**: Rust backend failed to establish SSE connection

**Solution**: Add `onError` callback to see Tauri command errors:

```typescript
useTauriSse({
  calendarId,
  onError: (error, context) => {
    if (context === "start_sse") {
      console.error("Failed to start Tauri SSE:", error)
    }
  },
})
```

## Future Improvements

- [ ] Add `onReconnect` callback for reconnection notifications
- [ ] Support custom exponential backoff parameters
- [ ] Add connection quality metrics (latency, success rate)
- [ ] Implement heartbeat timeout detection
- [ ] Add debug mode with verbose logging
