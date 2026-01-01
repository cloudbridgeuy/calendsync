/**
 * Pure sync operations.
 * These functions handle pending operation queue management.
 */

import type { LocalEntry, PendingOperation } from "./types"

/**
 * Check if an operation should be retried.
 * Returns true if retry_count is less than maxRetries.
 */
export function shouldRetry(op: PendingOperation, maxRetries: number): boolean {
  return op.retryCount < maxRetries
}

/**
 * Increment the retry count of a pending operation.
 * Returns a new operation with retry_count + 1.
 */
export function incrementRetry(op: PendingOperation): PendingOperation {
  return {
    ...op,
    retryCount: op.retryCount + 1,
  }
}

/**
 * Sort pending operations by creation time (oldest first).
 * Ensures operations are processed in the order they were created.
 */
export function sortByCreatedAt(ops: PendingOperation[]): PendingOperation[] {
  return [...ops].sort((a, b) => {
    const timeA = new Date(a.createdAt).getTime()
    const timeB = new Date(b.createdAt).getTime()
    return timeA - timeB
  })
}

/**
 * Mark a local entry as synced.
 * Sets sync_status to "synced" and clears pending_operation.
 */
export function markAsSynced(entry: LocalEntry): LocalEntry {
  return {
    ...entry,
    syncStatus: "synced",
    pendingOperation: null,
    lastSyncError: undefined,
  }
}

/**
 * Mark a local entry as having a sync conflict.
 * Sets sync_status to "conflict" and stores the error message.
 */
export function markAsConflict(entry: LocalEntry, error: string): LocalEntry {
  return {
    ...entry,
    syncStatus: "conflict",
    lastSyncError: error,
  }
}

/**
 * Create a new pending operation.
 * Initializes with retry_count of 0 and current timestamp.
 */
export function createPendingOperation(
  entryId: string,
  operation: PendingOperation["operation"],
  payload: PendingOperation["payload"],
): Omit<PendingOperation, "id"> {
  return {
    entryId,
    operation,
    payload,
    createdAt: new Date().toISOString(),
    retryCount: 0,
    lastError: null,
  }
}

/**
 * Set the last error on a pending operation.
 */
export function setOperationError(op: PendingOperation, error: string): PendingOperation {
  return {
    ...op,
    lastError: error,
  }
}
