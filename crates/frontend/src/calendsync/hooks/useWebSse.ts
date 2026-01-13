/**
 * Web SSE hook for browser-based calendar updates.
 *
 * This hook manages SSE connections using native EventSource and updates
 * Dexie directly when receiving events. This enables:
 * - Persistent storage of SSE updates across page refreshes
 * - Automatic sync confirmation (marks entries as synced)
 * - Last event ID tracking for reconnection catch-up
 *
 * Data flow:
 * 1. SSE event received from server via EventSource
 * 2. Entry updated in Dexie with syncStatus: "synced"
 * 3. last_event_id saved to sync_state table
 * 4. useLiveQuery in useOfflineCalendar reactively updates UI
 *
 * For Tauri desktop apps, use useTauriSse instead.
 */

import {
  buildSseUrl,
  calculateReconnectDelay,
  MAX_RECONNECT_ATTEMPTS,
  parseEventData,
} from "@core/sse/connection"
import type {
  BaseSseConfig,
  BaseSseResult,
  EntryAddedEvent,
  EntryDeletedEvent,
  EntryUpdatedEvent,
} from "@core/sse/types"
import { useLiveQuery } from "dexie-react-hooks"
import { useCallback, useEffect, useRef } from "react"

import { db } from "../db"
import { useConnectionManager, useDexieHandlers } from "./sse"
import { getControlPlaneUrl } from "./useApi"

/**
 * Configuration for the useWebSse hook.
 * Uses BaseSseConfig from core/sse/types.
 */
export type UseWebSseConfig = BaseSseConfig

/**
 * Result from useWebSse hook.
 * Uses BaseSseResult from core/sse/types.
 */
export type UseWebSseResult = BaseSseResult

/**
 * Hook for managing SSE connection in web browsers.
 *
 * Uses native EventSource API to connect to SSE endpoint and:
 * - Updates Dexie directly instead of React state
 * - Tracks last_event_id in IndexedDB for reconnection
 * - Marks entries as synced when confirmed by server
 *
 * @example
 * ```typescript
 * function MyCalendar({ calendarId }) {
 *   const { connectionState } = useWebSse({
 *     calendarId,
 *     onEntryAdded: (entry) => showNotification(`Added: ${entry.title}`),
 *   })
 *
 *   return (
 *     <div>
 *       {connectionState === "connected" && <span>Live</span>}
 *       {connectionState === "disconnected" && <span>Offline</span>}
 *     </div>
 *   )
 * }
 * ```
 */
export function useWebSse(config: UseWebSseConfig): UseWebSseResult {
  const {
    calendarId,
    enabled = typeof window !== "undefined",
    onEntryAdded,
    onEntryUpdated,
    onEntryDeleted,
    onConnectionChange,
    onError,
  } = config

  // Use shared connection manager for state management
  const { connectionState, updateConnectionState } = useConnectionManager({
    onConnectionChange,
  })

  // Use shared Dexie handlers for SSE event processing
  const {
    handleEntryAdded: dexieHandleAdded,
    handleEntryUpdated: dexieHandleUpdated,
    handleEntryDeleted: dexieHandleDeleted,
  } = useDexieHandlers()

  // Refs for EventSource and reconnection
  const eventSourceRef = useRef<EventSource | null>(null)
  const reconnectAttemptsRef = useRef<number>(0)
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Store event-specific callbacks in refs to avoid reconnecting on callback changes
  const eventCallbacksRef = useRef({
    onEntryAdded,
    onEntryUpdated,
    onEntryDeleted,
    onError,
  })

  // Update callback refs
  useEffect(() => {
    eventCallbacksRef.current = {
      onEntryAdded,
      onEntryUpdated,
      onEntryDeleted,
      onError,
    }
  }, [onEntryAdded, onEntryUpdated, onEntryDeleted, onError])

  // Load last event ID from Dexie reactively
  const syncState = useLiveQuery(() => db.sync_state.get(calendarId), [calendarId])

  const lastEventId = syncState?.lastEventId ?? null

  // Use a ref for the current lastEventId to avoid reconnection loop
  const lastEventIdRef = useRef<string | null>(null)

  // Update ref when syncState changes, but only if we're not currently connecting
  useEffect(() => {
    if (syncState?.lastEventId && connectionState !== "connecting") {
      lastEventIdRef.current = syncState.lastEventId
    }
  }, [syncState?.lastEventId, connectionState])

  /**
   * Save the last event ID to sync state.
   */
  const saveLastEventId = useCallback(
    async (eventId: string) => {
      lastEventIdRef.current = eventId
      await db.sync_state.put({
        calendarId,
        lastEventId: eventId,
        lastFullSync: syncState?.lastFullSync ?? null,
      })
    },
    [calendarId, syncState?.lastFullSync],
  )

  /**
   * Handle entry_added event.
   * Updates Dexie via shared handler, then calls user callback.
   */
  const handleEntryAdded = useCallback(
    async (data: EntryAddedEvent, eventId: string) => {
      await dexieHandleAdded(data.entry)
      await saveLastEventId(eventId)
      eventCallbacksRef.current.onEntryAdded?.(data.entry, data.date)
    },
    [dexieHandleAdded, saveLastEventId],
  )

  /**
   * Handle entry_updated event.
   * Updates Dexie via shared handler, then calls user callback.
   */
  const handleEntryUpdated = useCallback(
    async (data: EntryUpdatedEvent, eventId: string) => {
      await dexieHandleUpdated(data.entry)
      await saveLastEventId(eventId)
      eventCallbacksRef.current.onEntryUpdated?.(data.entry, data.date)
    },
    [dexieHandleUpdated, saveLastEventId],
  )

  /**
   * Handle entry_deleted event.
   * Updates Dexie via shared handler, then calls user callback.
   */
  const handleEntryDeleted = useCallback(
    async (data: EntryDeletedEvent, eventId: string) => {
      await dexieHandleDeleted(data.entry_id)
      await saveLastEventId(eventId)
      eventCallbacksRef.current.onEntryDeleted?.(data.entry_id, data.date)
    },
    [dexieHandleDeleted, saveLastEventId],
  )

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
    const sseUrl = buildSseUrl(baseUrl, calendarId, lastEventIdRef.current)

    const eventSource = new EventSource(sseUrl)
    eventSourceRef.current = eventSource

    // Handle connection open
    eventSource.onopen = () => {
      reconnectAttemptsRef.current = 0
      updateConnectionState("connected")
    }

    // Helper to create event listeners with error handling
    const createEventHandler = <T>(
      eventType: string,
      handler: (data: T, eventId: string) => Promise<void>,
    ) => {
      return (e: MessageEvent) => {
        const data = parseEventData<T>(e.data)
        if (!data) {
          const error = new Error(`Failed to parse ${eventType} event`)
          eventCallbacksRef.current.onError?.(error, `parse_${eventType}`)
          return
        }

        handler(data, e.lastEventId).catch((err) => {
          const error = err instanceof Error ? err : new Error(String(err))
          eventCallbacksRef.current.onError?.(error, `handle_${eventType}`)
        })
      }
    }

    // Handle SSE events
    eventSource.addEventListener(
      "entry_added",
      createEventHandler<EntryAddedEvent>("entry_added", handleEntryAdded),
    )
    eventSource.addEventListener(
      "entry_updated",
      createEventHandler<EntryUpdatedEvent>("entry_updated", handleEntryUpdated),
    )
    eventSource.addEventListener(
      "entry_deleted",
      createEventHandler<EntryDeletedEvent>("entry_deleted", handleEntryDeleted),
    )

    // Handle errors and reconnection
    eventSource.onerror = () => {
      eventSource.close()
      eventSourceRef.current = null

      // Check if we should reconnect
      if (reconnectAttemptsRef.current < MAX_RECONNECT_ATTEMPTS) {
        updateConnectionState("disconnected")
        reconnectAttemptsRef.current++

        // Schedule reconnection with exponential backoff
        const delay = calculateReconnectDelay(reconnectAttemptsRef.current)
        reconnectTimeoutRef.current = setTimeout(() => {
          connect()
        }, delay)
      } else {
        updateConnectionState("error")
        const error = new Error("Max reconnection attempts reached")
        eventCallbacksRef.current.onError?.(error, "max_reconnect_attempts")
      }
    }
  }, [
    calendarId,
    enabled,
    updateConnectionState,
    handleEntryAdded,
    handleEntryUpdated,
    handleEntryDeleted,
  ])

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
    lastEventId,
  }
}
