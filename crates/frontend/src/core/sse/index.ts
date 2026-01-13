/**
 * Core SSE module - types and pure functions for SSE handling.
 * This is the Functional Core for SSE-related logic.
 */

// Re-export connection functions
export {
  buildSseUrl,
  calculateReconnectDelay,
  MAX_RECONNECT_ATTEMPTS,
  parseEventData,
  RECONNECT_DELAY_MS,
  shouldReconnect,
} from "./connection"

// Re-export SSE sync functions
export type { SyncAction, UpdateSyncAction } from "./sync"
export { determineSyncAction, determineUpdateSyncAction } from "./sync"

// Re-export SSE types
export type {
  EntryAddedEvent,
  EntryDeletedEvent,
  EntryUpdatedEvent,
  SseConnectionState,
  SseEvent,
  SseEventType,
} from "./types"
