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
