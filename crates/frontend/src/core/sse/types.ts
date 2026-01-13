/**
 * SSE (Server-Sent Events) types - pure data types with no side effects.
 * This is the Functional Core for SSE-related types.
 */

import type { ServerEntry } from "../calendar/types"

/**
 * SSE connection states.
 */
export type SseConnectionState = "disconnected" | "connecting" | "connected" | "error"

/**
 * SSE event types from the server.
 */
export type SseEventType = "entry_added" | "entry_updated" | "entry_deleted"

/**
 * Base structure for all SSE events.
 */
interface SseEventBase {
  type: SseEventType
  date: string
}

/**
 * Event emitted when an entry is added to a calendar.
 */
export interface EntryAddedEvent extends SseEventBase {
  type: "entry_added"
  entry: ServerEntry
}

/**
 * Event emitted when an entry is updated in a calendar.
 */
export interface EntryUpdatedEvent extends SseEventBase {
  type: "entry_updated"
  entry: ServerEntry
}

/**
 * Event emitted when an entry is deleted from a calendar.
 */
export interface EntryDeletedEvent extends SseEventBase {
  type: "entry_deleted"
  entry_id: string
}

/**
 * Union of all SSE event types.
 */
export type SseEvent = EntryAddedEvent | EntryUpdatedEvent | EntryDeletedEvent

/**
 * Base configuration for SSE hooks (web and Tauri).
 * This interface defines the common configuration options shared across platforms.
 */
export interface BaseSseConfig {
  /** Calendar ID to subscribe to */
  calendarId: string
  /** Whether SSE is enabled (default: true) */
  enabled?: boolean
  /** Callback when an entry is added (for notifications, etc.) */
  onEntryAdded?: (entry: ServerEntry, date: string) => void
  /** Callback when an entry is updated (for notifications, etc.) */
  onEntryUpdated?: (entry: ServerEntry, date: string) => void
  /** Callback when an entry is deleted (for notifications, etc.) */
  onEntryDeleted?: (entryId: string, date: string) => void
  /** Callback when connection state changes */
  onConnectionChange?: (state: SseConnectionState) => void
  /** Callback when an error occurs */
  onError?: (error: Error, context: string) => void
}

/**
 * Base result from SSE hooks (web and Tauri).
 * This interface defines the common return value shared across platforms.
 */
export interface BaseSseResult {
  /** Current SSE connection state */
  connectionState: SseConnectionState
  /** Manually reconnect to SSE */
  reconnect: () => void
  /** Manually disconnect from SSE */
  disconnect: () => void
  /** Last event ID received (for debugging) */
  lastEventId: string | null
}
