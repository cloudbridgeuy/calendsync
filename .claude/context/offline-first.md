# Offline-First Architecture

This document describes the offline-first system for calendsync, enabling users to create, edit, and delete calendar entries while offline. Changes sync automatically when connectivity resumes.

## Overview

The system uses Dexie.js (IndexedDB wrapper) for client-side storage with Last-Write-Wins (LWW) conflict resolution. The backend remains the source of truth, broadcasting confirmed changes via SSE.

### Key Principles

1. **Optimistic UI**: Show changes immediately, mark as "pending" until server confirms
2. **Backend authority**: Server resolves conflicts and broadcasts final state via SSE
3. **Graceful degradation**: If IndexedDB is cleared, re-sync from server
4. **Minimal UI indicators**: Subtle sync status without cluttering the interface

## Architecture

```
+----------------------------------------------------------------------+
|                           CLIENT (React)                              |
+----------------------------------------------------------------------+
|                                                                       |
|  +--------------+    +------------------+    +---------------+        |
|  |   UI Layer   |--->| useOfflineCalendar|--->|   Dexie.js   |        |
|  |  (Calendar)  |<---|      Hook        |<---|  (IndexedDB) |        |
|  +--------------+    +------------------+    +---------------+        |
|         |                    |                      ^                 |
|         |                    v                      |                 |
|         |             +------------+                |                 |
|         |             | SyncEngine |----------------+                 |
|         |             +------------+                                  |
|         |                    |                                        |
|         v                    v                                        |
|  +--------------+    +---------------+                                |
|  |    useSse    |<---|   HTTP API    |                                |
|  +--------------+    +---------------+                                |
|         |                    |                                        |
+---------|--------------------|-----------------------------------------+
          | SSE events         | POST/PUT/DELETE
          v                    v
+----------------------------------------------------------------------+
|                           SERVER (Rust)                               |
+----------------------------------------------------------------------+
|  +--------------+    +--------------+    +---------------+            |
|  | SSE Handler  |<---| Entry CRUD   |--->|  Repository   |            |
|  +--------------+    +--------------+    +---------------+            |
+----------------------------------------------------------------------+
```

## Key Files and Responsibilities

### Database Layer

| File | Responsibility |
|------|----------------|
| `crates/frontend/src/calendsync/db/index.ts` | Dexie database schema and singleton instance |
| `crates/frontend/src/core/sync/types.ts` | Type definitions: `LocalEntry`, `PendingOperation`, `SyncState`, `SyncStatus` |

### Pure Functions (Functional Core)

| File | Responsibility |
|------|----------------|
| `crates/frontend/src/core/sync/operations.ts` | Pure sync operations: `shouldRetry`, `incrementRetry`, `sortByCreatedAt`, `markAsConflict` |
| `crates/frontend/src/core/sync/transformations.ts` | Form data to entry transformations: `formDataToEntry`, `deriveEntryType` |

### Sync Engine (Imperative Shell)

| File | Responsibility |
|------|----------------|
| `crates/frontend/src/calendsync/sync/engine.ts` | SyncEngine class: queues operations, processes pending, handles online/offline events |

### React Hooks

| File | Responsibility |
|------|----------------|
| `crates/frontend/src/calendsync/hooks/useOfflineCalendar.ts` | Main hook for offline-first calendar operations (CRUD, grouping, sync status) |
| `crates/frontend/src/calendsync/hooks/useSyncEngine.ts` | Hook exposing SyncEngine singleton with reactive state |
| `crates/frontend/src/calendsync/hooks/useCalendarState.ts` | **DEPRECATED** - Legacy hook for SSR-only calendars |

## Hook Architecture

The offline-first implementation uses a layered hook architecture:

### View Layer: useCalendarState
- Navigation state (centerDate, visibleDays)
- Flash animations for entry changes
- Toast notifications
- SSE connection lifecycle
- **NOT deprecated** - still needed for view concerns

### Data Layer: useOfflineCalendar
- Dexie/IndexedDB persistence
- Sync status tracking
- Reactive queries via useLiveQuery
- CRUD operations with optimistic updates

### Sync Layer: useSseWithOffline + useSyncEngine
- SSE event processing
- Pending operation queue
- Conflict resolution callbacks

### Data Flow

```
User Action → useOfflineCalendar (write to Dexie)
                    ↓
           useSyncEngine (queue operation)
                    ↓
           HTTP to Server → SSE broadcast
                    ↓
           useSseWithOffline (update Dexie, syncStatus: "synced")
                    ↓
           Callback to useCalendarState (flash animation, toast)
                    ↓
           useLiveQuery triggers re-render
```

### Why Both Hooks?

This is intentional separation of concerns:
- **Data persistence** should not know about animations
- **View state** should not manage IndexedDB
- **Callbacks** bridge the layers without tight coupling

## Data Flow

### Creating an Entry (Offline Capable)

1. User submits entry form
2. `useOfflineCalendar.createEntry()` generates temp UUID
3. Entry written to Dexie with `syncStatus: "pending"` and `pendingOperation: "create"`
4. UI immediately shows entry (optimistic)
5. `useSyncEngine.queueOperation()` stores operation in `pending_operations` table
6. If online, `SyncEngine.syncPending()` POSTs to server
7. Server stores entry, broadcasts `entry_added` via SSE
8. SSE handler updates entry to `syncStatus: "synced"`
9. UI removes "pending" indicator

### Receiving SSE Event (Another Device)

1. SSE event received by `useSse` hook
2. Entry written/updated in Dexie with `syncStatus: "synced"`
3. Dexie's `useLiveQuery` triggers automatic UI re-render

### Coming Online After Offline Changes

1. Browser fires `online` event
2. `SyncEngine.handleOnline()` triggers `syncPending()`
3. Operations processed in order (oldest first)
4. Failed operations retry up to 3 times
5. After max retries, entry marked as `syncStatus: "conflict"`

## Dexie Schema

```typescript
// Tables and indexes
entries: "id, calendarId, startDate, [calendarId+startDate], syncStatus"
pending_operations: "id, entryId, createdAt"
sync_state: "calendarId"
```

### LocalEntry Fields

Extends `ServerEntry` with sync tracking:

```typescript
interface LocalEntry extends ServerEntry {
  syncStatus: "synced" | "pending" | "conflict"
  localUpdatedAt: string        // ISO timestamp of last local change
  pendingOperation: "create" | "update" | "delete" | null
  lastSyncError?: string        // Error message if status is "conflict"
}
```

### PendingOperation Fields

```typescript
interface PendingOperation {
  id: string                    // UUID
  entryId: string               // Entry this operation affects
  operation: "create" | "update" | "delete"
  payload: Partial<ServerEntry> | null
  createdAt: string             // ISO timestamp for ordering
  retryCount: number            // Incremented on failure
  lastError: string | null      // Last error message
}
```

## Testing Offline Scenarios

### Manual Testing

1. **Simulate offline**: Open DevTools > Network tab > Throttle dropdown > Offline

2. **Create entry while offline**:
   - Create entry, verify it appears immediately with pending indicator
   - Check IndexedDB in DevTools > Application > IndexedDB > calendsync
   - Verify `pending_operations` table has the operation

3. **Go online**:
   - Disable offline mode
   - Verify entry syncs (pending indicator removed)
   - Verify `pending_operations` table is empty

4. **Test conflict resolution**:
   - Create entry offline
   - Wait for sync
   - If server rejects (e.g., validation error), verify entry shows conflict status

### Automated Testing

Pure functions in `core/sync/operations.ts` can be unit tested without mocks:

```typescript
import { shouldRetry, incrementRetry, sortByCreatedAt } from "@core/sync/operations"

describe("shouldRetry", () => {
  it("returns true when retryCount < maxRetries", () => {
    const op = { retryCount: 2 } as PendingOperation
    expect(shouldRetry(op, 3)).toBe(true)
  })
})
```

## Troubleshooting

### Entries Not Syncing

1. **Check online status**: `navigator.onLine` should be `true`
2. **Check pending operations**: DevTools > Application > IndexedDB > calendsync > pending_operations
3. **Check console for errors**: SyncEngine logs failures with error messages
4. **Check lastError on operation**: Failed operations store the error message

### Entries Stuck in "Pending"

1. **Verify SSE connection**: Look for SSE connection in DevTools > Network
2. **Check server logs**: Server should broadcast events after mutations
3. **Manual sync trigger**: Call `syncNow()` from useSyncEngine hook

### Conflict Status

Entries marked as "conflict" have failed after 3 retry attempts:

1. Check `lastSyncError` field for the error message
2. Common causes:
   - Server validation errors (invalid dates, missing fields)
   - Network timeouts
   - Server-side errors

### Clear Local Data

To reset local state (useful for debugging):

```javascript
// In browser console
indexedDB.deleteDatabase("calendsync")
// Then refresh the page
```

## Platform Considerations

### Browser

- IndexedDB via Dexie.js works natively
- Storage may be evicted under pressure; re-sync on eviction

### Tauri Desktop (macOS)

- WebView uses standard IndexedDB
- Same Dexie code works without modification
- Storage more reliable than browser

### Tauri iOS

- WKWebView supports IndexedDB (iOS 17+)
- 15% of disk quota per origin for non-browser apps
- Handle potential data loss gracefully

## Future Enhancements

These are explicitly out of scope for the initial implementation:

1. **Field-level LWW**: Track timestamps per field instead of per entry
2. **Background sync**: Use Service Worker for sync when app is closed
3. **Conflict UI**: Show diff and let user choose when conflicts occur
4. **Delta sync**: Only fetch changes since last sync instead of full refresh
