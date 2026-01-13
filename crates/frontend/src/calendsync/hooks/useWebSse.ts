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

import type { ServerEntry } from "@core/calendar/types"
import { MAX_RECONNECT_ATTEMPTS, RECONNECT_DELAY_MS } from "@core/sse/connection"
import type {
  EntryAddedEvent,
  EntryDeletedEvent,
  EntryUpdatedEvent,
  SseConnectionState,
} from "@core/sse/types"
import { useLiveQuery } from "dexie-react-hooks"
import { useCallback, useEffect, useRef } from "react"

import { db } from "../db"
import { useConnectionManager, useDexieHandlers } from "./sse"
import { getControlPlaneUrl } from "./useApi"

/** Configuration for the useWebSse hook */
export interface UseWebSseConfig {
  /** Calendar ID to subscribe to */
  calendarId: string
  /** Whether SSE is enabled (default: true, false on server) */
  enabled?: boolean
  /** Callback when an entry is added (for notifications, etc.) */
  onEntryAdded?: (entry: ServerEntry, date: string) => void
  /** Callback when an entry is updated (for notifications, etc.) */
  onEntryUpdated?: (entry: ServerEntry, date: string) => void
  /** Callback when an entry is deleted (for notifications, etc.) */
  onEntryDeleted?: (entryId: string, date: string) => void
  /** Callback when connection state changes */
  onConnectionChange?: (state: SseConnectionState) => void
}

/** Result from useWebSse hook */
export interface UseWebSseResult {
  /** Current SSE connection state */
  connectionState: SseConnectionState
  /** Manually reconnect to SSE */
  reconnect: () => void
  /** Manually disconnect from SSE */
  disconnect: () => void
  /** Last event ID received (for debugging) */
  lastEventId: string | null
}

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
  })

  // Update callback refs
  useEffect(() => {
    eventCallbacksRef.current = {
      onEntryAdded,
      onEntryUpdated,
      onEntryDeleted,
    }
  }, [onEntryAdded, onEntryUpdated, onEntryDeleted])

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
      try {
        const data = JSON.parse(e.data) as EntryAddedEvent
        handleEntryAdded(data, e.lastEventId).catch((err) => {
          console.error("Failed to handle entry_added event:", err)
        })
      } catch (err) {
        console.error("Failed to parse entry_added event:", err)
      }
    })

    // Handle entry_updated events
    eventSource.addEventListener("entry_updated", (e: MessageEvent) => {
      try {
        const data = JSON.parse(e.data) as EntryUpdatedEvent
        handleEntryUpdated(data, e.lastEventId).catch((err) => {
          console.error("Failed to handle entry_updated event:", err)
        })
      } catch (err) {
        console.error("Failed to parse entry_updated event:", err)
      }
    })

    // Handle entry_deleted events
    eventSource.addEventListener("entry_deleted", (e: MessageEvent) => {
      try {
        const data = JSON.parse(e.data) as EntryDeletedEvent
        handleEntryDeleted(data, e.lastEventId).catch((err) => {
          console.error("Failed to handle entry_deleted event:", err)
        })
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
