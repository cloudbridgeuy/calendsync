/**
 * SSE (Server-Sent Events) hook for real-time calendar updates.
 *
 * Connects to the SSE endpoint and handles:
 * - Connection management with automatic reconnection
 * - Event parsing (entry_added, entry_updated, entry_deleted)
 * - Last event ID tracking for reconnection catch-up
 */

import type { ServerEntry } from "@core/calendar/types"
import { useCallback, useEffect, useRef, useState } from "react"
import type { SseConnectionState } from "../types"
import { getControlPlaneUrl } from "./useApi"

/** SSE event types from the server */
export type SseEventType = "entry_added" | "entry_updated" | "entry_deleted"

/** Base SSE event data structure */
interface SseEventBase {
  type: SseEventType
  date: string
}

/** Entry added event */
export interface EntryAddedEvent extends SseEventBase {
  type: "entry_added"
  entry: ServerEntry
}

/** Entry updated event */
export interface EntryUpdatedEvent extends SseEventBase {
  type: "entry_updated"
  entry: ServerEntry
}

/** Entry deleted event */
export interface EntryDeletedEvent extends SseEventBase {
  type: "entry_deleted"
  entry_id: string
}

/** Union of all SSE event types */
export type SseEvent = EntryAddedEvent | EntryUpdatedEvent | EntryDeletedEvent

/** Callback for handling SSE events */
export type SseEventHandler = (event: SseEvent) => void

/** Configuration for the useSse hook */
export interface UseSseConfig {
  /** Calendar ID to subscribe to */
  calendarId: string
  /** Callback when an entry is added */
  onEntryAdded?: (entry: ServerEntry, date: string) => void
  /** Callback when an entry is updated */
  onEntryUpdated?: (entry: ServerEntry, date: string) => void
  /** Callback when an entry is deleted */
  onEntryDeleted?: (entryId: string, date: string) => void
  /** Callback for any event (for debugging or logging) */
  onEvent?: SseEventHandler
  /** Callback when connection state changes */
  onConnectionChange?: (state: SseConnectionState) => void
  /** Whether SSE is enabled (default: true, false on server) */
  enabled?: boolean
}

/** Return type for the useSse hook */
export interface UseSseResult {
  /** Current connection state */
  connectionState: SseConnectionState
  /** Manually reconnect to SSE */
  reconnect: () => void
  /** Manually disconnect from SSE */
  disconnect: () => void
}

/** Reconnection delay in milliseconds */
const RECONNECT_DELAY_MS = 3000

/** Maximum reconnection attempts before giving up */
const MAX_RECONNECT_ATTEMPTS = 5

/**
 * Hook for managing SSE connection and events.
 *
 * @param config - SSE configuration
 * @returns SSE state and control methods
 */
export function useSse(config: UseSseConfig): UseSseResult {
  const {
    calendarId,
    onEntryAdded,
    onEntryUpdated,
    onEntryDeleted,
    onEvent,
    onConnectionChange,
    enabled = typeof window !== "undefined", // Disable on server
  } = config

  const [connectionState, setConnectionState] = useState<SseConnectionState>("disconnected")

  // Refs for EventSource and reconnection
  const eventSourceRef = useRef<EventSource | null>(null)
  const lastEventIdRef = useRef<string | null>(null)
  const reconnectAttemptsRef = useRef<number>(0)
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Store callbacks in refs to avoid reconnecting on callback changes
  const callbacksRef = useRef({
    onEntryAdded,
    onEntryUpdated,
    onEntryDeleted,
    onEvent,
    onConnectionChange,
  })

  // Update callback refs
  useEffect(() => {
    callbacksRef.current = {
      onEntryAdded,
      onEntryUpdated,
      onEntryDeleted,
      onEvent,
      onConnectionChange,
    }
  }, [onEntryAdded, onEntryUpdated, onEntryDeleted, onEvent, onConnectionChange])

  /**
   * Update connection state and notify callback.
   */
  const updateConnectionState = useCallback((state: SseConnectionState) => {
    setConnectionState(state)
    callbacksRef.current.onConnectionChange?.(state)
  }, [])

  /**
   * Disconnect from SSE.
   */
  const disconnect = useCallback(() => {
    // Clear reconnect timeout
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current)
      reconnectTimeoutRef.current = null
    }

    // Close EventSource
    if (eventSourceRef.current) {
      eventSourceRef.current.close()
      eventSourceRef.current = null
    }

    updateConnectionState("disconnected")
  }, [updateConnectionState])

  /**
   * Connect to SSE endpoint.
   */
  const connect = useCallback(() => {
    // Don't connect if disabled or already connected
    if (!enabled) return
    if (eventSourceRef.current?.readyState === EventSource.OPEN) return

    // Clean up existing connection
    if (eventSourceRef.current) {
      eventSourceRef.current.close()
    }

    updateConnectionState("connecting")

    // Build SSE URL with optional last event ID
    const baseUrl = getControlPlaneUrl()
    let sseUrl = `${baseUrl}/api/events?calendar_id=${calendarId}`
    if (lastEventIdRef.current) {
      sseUrl += `&last_event_id=${lastEventIdRef.current}`
    }

    const eventSource = new EventSource(sseUrl)
    eventSourceRef.current = eventSource

    // Handle connection open
    eventSource.onopen = () => {
      reconnectAttemptsRef.current = 0
      updateConnectionState("connected")
    }

    // Handle entry_added events
    eventSource.addEventListener("entry_added", (e: MessageEvent) => {
      lastEventIdRef.current = e.lastEventId
      try {
        const data = JSON.parse(e.data) as EntryAddedEvent
        callbacksRef.current.onEvent?.(data)
        callbacksRef.current.onEntryAdded?.(data.entry, data.date)
      } catch (err) {
        console.error("Failed to parse entry_added event:", err)
      }
    })

    // Handle entry_updated events
    eventSource.addEventListener("entry_updated", (e: MessageEvent) => {
      lastEventIdRef.current = e.lastEventId
      try {
        const data = JSON.parse(e.data) as EntryUpdatedEvent
        callbacksRef.current.onEvent?.(data)
        callbacksRef.current.onEntryUpdated?.(data.entry, data.date)
      } catch (err) {
        console.error("Failed to parse entry_updated event:", err)
      }
    })

    // Handle entry_deleted events
    eventSource.addEventListener("entry_deleted", (e: MessageEvent) => {
      lastEventIdRef.current = e.lastEventId
      try {
        const data = JSON.parse(e.data) as EntryDeletedEvent
        callbacksRef.current.onEvent?.(data)
        callbacksRef.current.onEntryDeleted?.(data.entry_id, data.date)
      } catch (err) {
        console.error("Failed to parse entry_deleted event:", err)
      }
    })

    // Handle errors and reconnection
    eventSource.onerror = () => {
      eventSource.close()
      eventSourceRef.current = null

      // Check if we should reconnect
      if (reconnectAttemptsRef.current < MAX_RECONNECT_ATTEMPTS) {
        updateConnectionState("disconnected")
        reconnectAttemptsRef.current++

        // Schedule reconnection
        reconnectTimeoutRef.current = setTimeout(() => {
          connect()
        }, RECONNECT_DELAY_MS)
      } else {
        updateConnectionState("error")
      }
    }
  }, [calendarId, enabled, updateConnectionState])

  /**
   * Reconnect to SSE (manual trigger).
   */
  const reconnect = useCallback(() => {
    reconnectAttemptsRef.current = 0
    disconnect()
    connect()
  }, [connect, disconnect])

  // Connect on mount and disconnect on unmount
  useEffect(() => {
    if (enabled) {
      connect()
    }

    return () => {
      disconnect()
    }
  }, [enabled, connect, disconnect])

  return {
    connectionState,
    reconnect,
    disconnect,
  }
}
