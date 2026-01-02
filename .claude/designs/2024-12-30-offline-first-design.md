# Offline-First Architecture Design

This document describes the offline-first feature for calendsync, enabling users to create, edit, and delete calendar entries while offline. Changes sync automatically when connectivity resumes.

## Overview

The design uses Dexie.js (IndexedDB wrapper) for client-side storage across all platforms: browser, Tauri desktop, and iOS. The backend remains the source of truth, with Last-Write-Wins (LWW) conflict resolution based on `updated_at` timestamps.

### Key Principles

1. **Optimistic UI**: Show changes immediately, mark as "pending" until confirmed
2. **Backend authority**: Server resolves conflicts and broadcasts final state via SSE
3. **Graceful degradation**: If IndexedDB is cleared, re-sync from server
4. **Minimal UI indicators**: Subtle sync status without cluttering the interface

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CLIENT (React)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                   │
│  │   UI Layer   │───▶│  useOffline  │───▶│   Dexie.js   │                   │
│  │  (Calendar)  │◀───│    Hook      │◀───│  (IndexedDB) │                   │
│  └──────────────┘    └──────────────┘    └──────────────┘                   │
│         │                   │                    ▲                           │
│         │                   ▼                    │                           │
│         │            ┌──────────────┐            │                           │
│         │            │  SyncEngine  │────────────┤                           │
│         │            └──────────────┘            │                           │
│         │                   │                    │                           │
│         ▼                   ▼                    │                           │
│  ┌──────────────┐    ┌──────────────┐            │                           │
│  │    useSse    │◀───│   HTTP API   │            │                           │
│  └──────────────┘    └──────────────┘            │                           │
│         │                   │                    │                           │
└─────────│───────────────────│────────────────────│───────────────────────────┘
          │  SSE events       │  POST/PUT/DELETE   │  Apply confirmed state
          ▼                   ▼                    │
┌─────────────────────────────────────────────────────────────────────────────┐
│                              SERVER (Rust)                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                   │
│  │ SSE Handler  │◀───│   Handlers   │───▶│  Repository  │                   │
│  └──────────────┘    └──────────────┘    └──────────────┘                   │
│                             │                    │                           │
│                      ┌──────────────┐    ┌──────────────┐                   │
│                      │   LWW Merge  │    │   Storage    │                   │
│                      └──────────────┘    └──────────────┘                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Data Flow

**Creating an entry offline:**

1. User creates entry in UI
2. `useOfflineCalendar` writes to Dexie with `sync_status: "pending"`
3. UI immediately shows entry (optimistic)
4. `SyncEngine` queues the create operation
5. When online, `SyncEngine` POSTs to server
6. Server stores entry, broadcasts `entry_added` via SSE
7. SSE handler updates Dexie to `sync_status: "synced"`
8. UI removes "pending" indicator

**Receiving SSE event from another device:**

1. `useSseWithOffline` receives event
2. Updates Dexie directly (already confirmed by server)
3. `useLiveQuery` triggers automatic UI re-render

## Dexie Database Schema

```typescript
// crates/frontend/src/calendsync/db/index.ts

import Dexie, { type EntityTable } from "dexie"

interface LocalEntry {
  // Core fields (mirror CalendarEntry)
  id: string
  calendar_id: string
  title: string
  start_date: string
  end_date: string
  kind: EntryKind
  color?: string
  description?: string
  location?: string
  created_at: string
  updated_at: string

  // Sync metadata
  sync_status: "synced" | "pending" | "conflict"
  local_updated_at: string
  pending_operation?: "create" | "update" | "delete"
}

interface PendingOperation {
  id: string
  entry_id: string
  operation: "create" | "update" | "delete"
  payload: unknown
  created_at: string
  retry_count: number
  last_error?: string
}

interface SyncState {
  calendar_id: string
  last_event_id: number
  last_full_sync: string
}

const db = new Dexie("calendsync") as Dexie & {
  entries: EntityTable<LocalEntry, "id">
  pending_operations: EntityTable<PendingOperation, "id">
  sync_state: EntityTable<SyncState, "calendar_id">
}

db.version(1).stores({
  entries: "id, calendar_id, start_date, [calendar_id+start_date], sync_status",
  pending_operations: "id, entry_id, created_at",
  sync_state: "calendar_id"
})

export { db }
export type { LocalEntry, PendingOperation, SyncState }
```

## SyncEngine

The SyncEngine orchestrates offline sync following Functional Core - Imperative Shell:

### Pure Functions (Functional Core)

```typescript
// crates/frontend/src/core/sync/operations.ts

interface PendingOperation {
  id: string
  entry_id: string
  operation: "create" | "update" | "delete"
  payload: unknown
  created_at: string
  retry_count: number
  last_error?: string
}

function shouldRetry(op: PendingOperation, maxRetries: number): boolean {
  return op.retry_count < maxRetries
}

function incrementRetry(op: PendingOperation): PendingOperation {
  return { ...op, retry_count: op.retry_count + 1 }
}

function sortByCreatedAt(ops: PendingOperation[]): PendingOperation[] {
  return [...ops].sort((a, b) =>
    new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
  )
}

export { shouldRetry, incrementRetry, sortByCreatedAt }
export type { PendingOperation }
```

### I/O Operations (Imperative Shell)

```typescript
// crates/frontend/src/calendsync/sync/engine.ts

class SyncEngine {
  private isOnline: boolean = navigator.onLine
  private isSyncing: boolean = false

  constructor(
    private db: CalendSyncDB,
    private api: ApiClient
  ) {
    window.addEventListener("online", () => this.onOnline())
    window.addEventListener("offline", () => this.onOffline())
  }

  async queueOperation(op: Omit<PendingOperation, "id" | "created_at" | "retry_count">) {
    const pending: PendingOperation = {
      ...op,
      id: crypto.randomUUID(),
      created_at: new Date().toISOString(),
      retry_count: 0
    }
    await this.db.pending_operations.add(pending)

    if (this.isOnline) {
      this.syncPending()
    }
  }

  private async onOnline() {
    this.isOnline = true
    await this.syncPending()
  }

  private async syncPending() {
    if (this.isSyncing) return
    this.isSyncing = true

    try {
      const pending = await this.db.pending_operations.toArray()
      const sorted = sortByCreatedAt(pending)

      for (const op of sorted) {
        const result = await this.executeOperation(op)
        if (result.success) {
          await this.db.pending_operations.delete(op.id)
        } else if (shouldRetry(op, 3)) {
          await this.db.pending_operations.put(incrementRetry(op))
        } else {
          await this.db.entries.update(op.entry_id, {
            sync_status: "conflict",
            last_error: result.error
          })
          await this.db.pending_operations.delete(op.id)
        }
      }
    } finally {
      this.isSyncing = false
    }
  }
}
```

## Backend LWW Merge

The server applies Last-Write-Wins conflict resolution:

```rust
// crates/core/src/calendar/merge.rs

use chrono::{DateTime, Utc};
use crate::calendar::types::CalendarEntry;

#[derive(Debug, Clone, PartialEq)]
pub enum MergeResult {
    ClientWins(CalendarEntry),
    ServerWins(CalendarEntry),
}

pub fn merge_entry(
    server_entry: &CalendarEntry,
    client_entry: &CalendarEntry,
) -> MergeResult {
    if client_entry.updated_at > server_entry.updated_at {
        MergeResult::ClientWins(client_entry.clone())
    } else {
        MergeResult::ServerWins(server_entry.clone())
    }
}
```

The handler uses this for updates:

```rust
// crates/calendsync/src/handlers/entries.rs (modified)

pub async fn update_entry(
    State(state): State<AppState>,
    Path(entry_id): Path<Uuid>,
    Json(request): Json<UpdateEntryRequest>,
) -> Result<Json<CalendarEntry>, AppError> {
    let current = state.entry_repo
        .get_entry(entry_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let proposed = build_entry_from_request(entry_id, &request);

    match merge_entry(&current, &proposed) {
        MergeResult::ClientWins(entry) => {
            state.entry_repo.update_entry(&entry).await?;
            Ok(Json(entry))
        }
        MergeResult::ServerWins(entry) => {
            Ok(Json(entry))
        }
    }
}
```

## React Hooks

### useOfflineCalendar

Main hook replacing `useCalendarState`:

```typescript
// crates/frontend/src/calendsync/hooks/useOfflineCalendar.ts

import { useLiveQuery } from "dexie-react-hooks"
import { db } from "../db"
import { useSyncEngine } from "./useSyncEngine"
import { groupEntriesByDate } from "../../core/calendar/entries"

export function useOfflineCalendar(calendarId: string) {
  const syncEngine = useSyncEngine()

  const entries = useLiveQuery(
    () => db.entries
      .where("calendar_id")
      .equals(calendarId)
      .toArray(),
    [calendarId]
  )

  const entriesByDate = useMemo(() => {
    if (!entries) return new Map()
    return groupEntriesByDate(entries)
  }, [entries])

  const createEntry = useCallback(async (data: CreateEntryData) => {
    const entry: LocalEntry = {
      id: crypto.randomUUID(),
      calendar_id: calendarId,
      ...data,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      sync_status: "pending",
      local_updated_at: new Date().toISOString(),
      pending_operation: "create"
    }

    await db.entries.add(entry)

    syncEngine.queueOperation({
      entry_id: entry.id,
      operation: "create",
      payload: toCreateRequest(entry)
    })
  }, [calendarId, syncEngine])

  return {
    entries,
    entriesByDate,
    createEntry,
    updateEntry,
    deleteEntry,
    isOnline: syncEngine.isOnline,
    pendingCount: syncEngine.pendingCount
  }
}
```

### useSseWithOffline

SSE handler that updates Dexie:

```typescript
// crates/frontend/src/calendsync/hooks/useSseWithOffline.ts

export function useSseWithOffline(calendarId: string) {
  const { lastEventId } = useSyncState(calendarId)

  useEffect(() => {
    const eventSource = new EventSource(
      `/api/events?calendar_id=${calendarId}&last_event_id=${lastEventId}`
    )

    eventSource.addEventListener("entry_added", async (e) => {
      const { entry } = JSON.parse(e.data)

      await db.entries.put({
        ...entry,
        sync_status: "synced",
        pending_operation: undefined
      })

      await db.sync_state.put({
        calendar_id: calendarId,
        last_event_id: parseInt(e.lastEventId)
      })
    })

    // Similar handlers for entry_updated, entry_deleted

    return () => eventSource.close()
  }, [calendarId, lastEventId])
}
```

### useInitialSync

Handles app initialization:

```typescript
// crates/frontend/src/calendsync/hooks/useInitialSync.ts

export function useInitialSync(calendarId: string) {
  const [isReady, setIsReady] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    async function initialize() {
      try {
        if (navigator.storage?.persist) {
          await navigator.storage.persist()
        }

        const syncState = await db.sync_state.get(calendarId)
        const localEntries = await db.entries
          .where("calendar_id").equals(calendarId)
          .count()

        if (localEntries === 0 || !syncState) {
          await performFullSync(calendarId)
        }

        setIsReady(true)
      } catch (err) {
        setError(err.message)
      }
    }

    initialize()
  }, [calendarId])

  return { isReady, error }
}
```

## UI Indicators

Minimal visual feedback for sync status:

```typescript
// components/Entry/Entry.tsx

export function Entry({ entry, ...props }: EntryProps) {
  const isPending = entry.sync_status === "pending"
  const hasConflict = entry.sync_status === "conflict"

  return (
    <div
      className={cn(
        "entry",
        isPending && "entry--pending",
        hasConflict && "entry--conflict"
      )}
      {...props}
    >
      {/* Existing content */}

      {isPending && (
        <span className="entry__sync-indicator" title="Syncing...">
          <SyncIcon className="animate-pulse" />
        </span>
      )}

      {hasConflict && (
        <span className="entry__conflict-indicator" title="Sync failed">
          <WarningIcon />
        </span>
      )}
    </div>
  )
}
```

```css
.entry--pending {
  opacity: 0.8;
}

.entry__sync-indicator {
  position: absolute;
  top: 2px;
  right: 2px;
  color: var(--color-muted);
}

.entry--conflict {
  border-left: 3px solid var(--color-warning);
}
```

## Testing Strategy

### TypeScript Unit Tests (Pure Functions)

```typescript
// core/sync/__tests__/operations.test.ts

describe("shouldRetry", () => {
  it("returns true when retry_count < maxRetries", () => {
    const op = { retry_count: 2 } as PendingOperation
    expect(shouldRetry(op, 3)).toBe(true)
  })

  it("returns false when retry_count >= maxRetries", () => {
    const op = { retry_count: 3 } as PendingOperation
    expect(shouldRetry(op, 3)).toBe(false)
  })
})
```

### Rust Unit Tests (Core Logic)

```rust
// crates/core/src/calendar/merge.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_wins_when_newer() {
        let server = entry_with_updated_at("2024-01-01T00:00:00Z");
        let client = entry_with_updated_at("2024-01-02T00:00:00Z");

        assert!(matches!(
            merge_entry(&server, &client),
            MergeResult::ClientWins(_)
        ));
    }
}
```

### Integration Tests

- SyncEngine with mock Dexie
- SSE handler updates to Dexie
- Initial sync flow
- Offline → online scenarios

## Files to Modify/Create

### New Files

| Path | Purpose |
|------|---------|
| `crates/frontend/src/calendsync/db/index.ts` | Dexie database schema |
| `crates/frontend/src/core/sync/types.ts` | Sync-related types |
| `crates/frontend/src/core/sync/operations.ts` | Pure sync functions |
| `crates/frontend/src/core/sync/__tests__/operations.test.ts` | Unit tests |
| `crates/frontend/src/calendsync/sync/engine.ts` | SyncEngine class |
| `crates/frontend/src/calendsync/hooks/useOfflineCalendar.ts` | Main offline hook |
| `crates/frontend/src/calendsync/hooks/useSseWithOffline.ts` | SSE + Dexie handler |
| `crates/frontend/src/calendsync/hooks/useInitialSync.ts` | Initialization hook |
| `crates/frontend/src/calendsync/hooks/useSyncEngine.ts` | SyncEngine hook |
| `crates/core/src/calendar/merge.rs` | LWW merge logic |
| `.claude/context/offline-first.md` | Context documentation |

### Modified Files

| Path | Changes |
|------|---------|
| `crates/frontend/package.json` | Add dexie, dexie-react-hooks |
| `crates/frontend/src/calendsync/hooks/useCalendarState.ts` | Replace with useOfflineCalendar |
| `crates/frontend/src/calendsync/contexts/CalendarContext.tsx` | Use offline hooks |
| `crates/frontend/src/calendsync/components/Entry/*.tsx` | Add sync indicators |
| `crates/frontend/src/calendsync/styles.css` | Add sync indicator styles |
| `crates/calendsync/src/handlers/entries.rs` | Add LWW merge to update |
| `crates/core/src/calendar/mod.rs` | Export merge module |
| `crates/core/src/calendar/requests.rs` | Add updated_at to UpdateEntryRequest |

## Platform Considerations

### Browser

- IndexedDB via Dexie.js works natively
- Request persistent storage with `navigator.storage.persist()`
- Gracefully handle storage eviction by re-syncing

### Tauri Desktop (macOS)

- WebView uses standard IndexedDB
- Same Dexie code works without modification
- Storage more reliable than mobile

### Tauri iOS

- WKWebView supports IndexedDB (iOS 17+)
- Request persistent storage to reduce eviction risk
- 15% of disk quota per origin for non-browser apps
- Handle potential data loss gracefully

## Future Enhancements

These are explicitly out of scope for the initial implementation:

1. **Field-level LWW**: Track timestamps per field instead of per entry
2. **Background sync**: Use Service Worker for sync when app is closed
3. **Conflict UI**: Show diff and let user choose when conflicts occur
4. **Multi-calendar sync**: Currently scoped to single calendar at a time
5. **Delta sync**: Only fetch changes since last sync instead of full refresh
