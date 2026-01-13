/**
 * Core sync module - types and pure functions for offline-first sync.
 * This is the Functional Core of the sync layer.
 */

// Re-export pure functions from operations
export {
  createPendingOperation,
  incrementRetry,
  markAsConflict,
  markAsSynced,
  setOperationError,
  shouldRetry,
  sortByCreatedAt,
} from "./operations"
// Re-export sync strategy functions
export type { SyncStrategy } from "./strategy"
export { decideSyncStrategy } from "./strategy"
export type { DerivedEntryType } from "./transformations"
// Re-export pure functions from transformations
export { deriveEntryType, formDataToEntry } from "./transformations"
// Re-export all types
export type {
  LocalEntry,
  PendingOperation,
  PendingOperationType,
  SyncResult,
  SyncState,
  SyncStatus,
} from "./types"
// Re-export pure functions from types
export {
  createLocalEntry,
  markEntryAsPending,
  serverToLocalEntry,
} from "./types"
