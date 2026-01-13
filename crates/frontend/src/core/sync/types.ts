/**
 * Sync types for offline-first calendar entries.
 * These types extend the server types with local sync tracking fields.
 */

import type { ServerEntry } from "../calendar/types"

/**
 * Sync status for local entries.
 * - synced: Entry matches the server version
 * - pending: Entry has local changes not yet sent to server
 * - conflict: Sync failed after max retries (requires manual resolution)
 */
export type SyncStatus = "synced" | "pending" | "conflict"

/**
 * Type of pending operation for an entry.
 * - create: Entry was created locally and needs to be sent to server
 * - update: Entry was modified locally and needs to be sent to server
 * - delete: Entry was deleted locally and needs to be deleted on server
 */
export type PendingOperationType = "create" | "update" | "delete"

/**
 * Local entry extends ServerEntry with sync tracking fields.
 * This is the primary entry type stored in IndexedDB.
 */
export interface LocalEntry extends ServerEntry {
  /** Current sync status */
  syncStatus: SyncStatus
  /** Timestamp of last local modification (ISO 8601) */
  localUpdatedAt: string
  /** Type of pending operation, if any */
  pendingOperation: PendingOperationType | null
  /** Last sync error message, if status is conflict */
  lastSyncError?: string
}

/**
 * Pending operation queued for sync.
 * Operations are processed in order and retried on failure.
 */
export interface PendingOperation {
  /** Unique identifier for this operation */
  id: string
  /** ID of the entry this operation affects */
  entryId: string
  /** Type of operation */
  operation: PendingOperationType
  /** Full payload for create/update operations */
  payload: Partial<ServerEntry> | null
  /** Timestamp when operation was created (ISO 8601) */
  createdAt: string
  /** Number of retry attempts */
  retryCount: number
  /** Last error message, if any */
  lastError: string | null
}

/**
 * Per-calendar sync state tracking.
 * Used to resume sync from the last known position.
 */
export interface SyncState {
  /** Calendar ID this state belongs to */
  calendarId: string
  /** Last SSE event ID received for incremental sync */
  lastEventId: string | null
  /** Timestamp of last full sync (ISO 8601) */
  lastFullSync: string | null
}

/**
 * Result of a sync operation.
 */
export interface SyncResult {
  /** Whether the sync was successful */
  success: boolean
  /** Number of entries synced */
  syncedCount: number
  /** Number of operations that failed */
  failedCount: number
  /** Error message if sync failed */
  error?: string
}

/**
 * Convert a ServerEntry to a LocalEntry with initial sync state.
 * Used when fetching entries from the server.
 */
export function serverToLocalEntry(entry: ServerEntry): LocalEntry {
  return {
    ...entry,
    syncStatus: "synced",
    localUpdatedAt: new Date().toISOString(),
    pendingOperation: null,
  }
}

/**
 * Create a new LocalEntry from form data (before server sync).
 * The entry starts in "pending" state with a "create" operation.
 */
export function createLocalEntry(entry: Omit<ServerEntry, "id">, tempId: string): LocalEntry {
  return {
    ...entry,
    id: tempId,
    syncStatus: "pending",
    localUpdatedAt: new Date().toISOString(),
    pendingOperation: "create",
  }
}

/**
 * Mark a LocalEntry as having pending changes.
 */
export function markEntryAsPending(entry: LocalEntry, operation: PendingOperationType): LocalEntry {
  return {
    ...entry,
    syncStatus: "pending",
    localUpdatedAt: new Date().toISOString(),
    pendingOperation: operation,
  }
}
