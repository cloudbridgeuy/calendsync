/**
 * SSE hook for offline-first calendar updates.
 *
 * This hook manages SSE connections and updates Dexie directly
 * (instead of React state) when receiving events. This enables:
 * - Persistent storage of SSE updates across page refreshes
 * - Automatic sync confirmation (marks entries as synced)
 * - Last event ID tracking for reconnection catch-up
 *
 * Data flow:
 * 1. SSE event received from server
 * 2. Entry updated in Dexie with syncStatus: "synced"
 * 3. last_event_id saved to sync_state table
 * 4. useLiveQuery in useOfflineCalendar reactively updates UI
 */

import type { ServerEntry } from "@core/calendar/types"
import { serverToLocalEntry } from "@core/sync/types"
import { useLiveQuery } from "dexie-react-hooks"
import { useCallback, useEffect, useRef, useState } from "react"

import { db } from "../db"
import type { SseConnectionState } from "../types"
import { getControlPlaneUrl } from "./useApi"
import type { EntryAddedEvent, EntryDeletedEvent, EntryUpdatedEvent } from "./useSse"

/** Configuration for the useSseWithOffline hook */
export interface UseSseWithOfflineConfig {
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

/** Result from useSseWithOffline hook */
export interface UseSseWithOfflineResult {
  /** Current SSE connection state */
  connectionState: SseConnectionState
  /** Manually reconnect to SSE */
  reconnect: () => void
  /** Manually disconnect from SSE */
  disconnect: () => void
  /** Last event ID received (for debugging) */
  lastEventId: string | null
}

/** Reconnection delay in milliseconds */
const RECONNECT_DELAY_MS = 3000

/** Maximum reconnection attempts before giving up */
const MAX_RECONNECT_ATTEMPTS = 5

/**
 * Hook for managing SSE connection with offline-first storage.
 *
 * Unlike the regular useSse hook, this hook:
 * - Updates Dexie directly instead of React state
 * - Tracks last_event_id in IndexedDB for reconnection
 * - Marks entries as synced when confirmed by server
 *
 * @example
 * ```typescript
 * function MyCalendar({ calendarId }) {
 *   const { connectionState } = useSseWithOffline({
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
export function useSseWithOffline(config: UseSseWithOfflineConfig): UseSseWithOfflineResult {
  const {
    calendarId,
    enabled = typeof window !== "undefined",
    onEntryAdded,
    onEntryUpdated,
    onEntryDeleted,
    onConnectionChange,
  } = config

  const [connectionState, setConnectionState] = useState<SseConnectionState>("disconnected")

  // Refs for EventSource and reconnection
  const eventSourceRef = useRef<EventSource | null>(null)
  const reconnectAttemptsRef = useRef<number>(0)
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Store callbacks in refs to avoid reconnecting on callback changes
  const callbacksRef = useRef({
    onEntryAdded,
    onEntryUpdated,
    onEntryDeleted,
    onConnectionChange,
  })

  // Update callback refs
  useEffect(() => {
    callbacksRef.current = {
      onEntryAdded,
      onEntryUpdated,
      onEntryDeleted,
      onConnectionChange,
    }
  }, [onEntryAdded, onEntryUpdated, onEntryDeleted, onConnectionChange])

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
   * Update connection state and notify callback.
   */
  const updateConnectionState = useCallback((state: SseConnectionState) => {
    setConnectionState(state)
    callbacksRef.current.onConnectionChange?.(state)
  }, [])

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
   * Adds or updates the entry in Dexie with synced status.
   */
  const handleEntryAdded = useCallback(
    async (data: EntryAddedEvent, eventId: string) => {
      const localEntry = serverToLocalEntry(data.entry)

      // Check if this is our own pending entry being confirmed
      const existing = await db.entries.get(data.entry.id)
      if (existing?.pendingOperation === "create") {
        // Our create was confirmed - mark as synced
        await db.entries.update(data.entry.id, {
          ...localEntry,
          syncStatus: "synced",
          pendingOperation: null,
        })
      } else {
        // New entry from another client - add it
        await db.entries.put(localEntry)
      }

      await saveLastEventId(eventId)
      callbacksRef.current.onEntryAdded?.(data.entry, data.date)
    },
    [saveLastEventId],
  )

  /**
   * Handle entry_updated event.
   * Updates the entry in Dexie with synced status.
   */
  const handleEntryUpdated = useCallback(
    async (data: EntryUpdatedEvent, eventId: string) => {
      const localEntry = serverToLocalEntry(data.entry)

      // Check if this is our own pending entry being confirmed
      const existing = await db.entries.get(data.entry.id)
      if (existing?.pendingOperation === "update") {
        // Our update was confirmed - mark as synced
        await db.entries.update(data.entry.id, {
          ...localEntry,
          syncStatus: "synced",
          pendingOperation: null,
        })
      } else {
        // Update from another client - apply it
        await db.entries.put(localEntry)
      }

      await saveLastEventId(eventId)
      callbacksRef.current.onEntryUpdated?.(data.entry, data.date)
    },
    [saveLastEventId],
  )

  /**
   * Handle entry_deleted event.
   * Removes the entry from Dexie.
   */
  const handleEntryDeleted = useCallback(
    async (data: EntryDeletedEvent, eventId: string) => {
      // Delete from local database
      await db.entries.delete(data.entry_id)

      await saveLastEventId(eventId)
      callbacksRef.current.onEntryDeleted?.(data.entry_id, data.date)
    },
    [saveLastEventId],
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
