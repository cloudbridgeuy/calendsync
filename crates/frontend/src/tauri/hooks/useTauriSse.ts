/**
 * Tauri SSE hook for desktop/mobile calendar updates.
 *
 * This hook manages SSE connections through Tauri's Rust backend and updates
 * Dexie directly when receiving events. This enables:
 * - Persistent storage of SSE updates across app restarts
 * - Automatic sync confirmation (marks entries as synced)
 * - Proper authentication via session cookies (handled by Rust)
 *
 * Data flow:
 * 1. Frontend calls start_sse command with calendar_id
 * 2. Rust backend connects to /api/events with session cookie
 * 3. Rust backend parses SSE messages and emits Tauri events
 * 4. This hook updates Dexie with syncStatus: "synced"
 * 5. useLiveQuery in useOfflineCalendar reactively updates UI
 *
 * For web browsers, use useWebSse instead.
 */

import type { ServerEntry } from "@core/calendar/types"
import type { BaseSseConfig, BaseSseResult, SseConnectionState } from "@core/sse/types"
import { useCallback, useEffect, useRef } from "react"

import { useConnectionManager, useDexieHandlers } from "../../calendsync/hooks/sse"

// Re-export types from core for consumers (preserves public API)
export type { SseConnectionState }

/**
 * Configuration for the useTauriSse hook.
 * Uses BaseSseConfig from core/sse/types.
 */
export type UseTauriSseConfig = BaseSseConfig

/**
 * Result from useTauriSse hook.
 * Uses BaseSseResult from core/sse/types.
 */
export type UseTauriSseResult = BaseSseResult

/**
 * Hook for managing SSE connection in Tauri desktop/mobile apps.
 *
 * Uses Tauri events from Rust backend and:
 * - Updates Dexie directly instead of React state
 * - Marks entries as synced when confirmed by server
 * - Handles authentication via session cookies (managed by Rust)
 *
 * **Note:** lastEventId tracking is handled by the Rust backend.
 * The frontend returns null as Dexie sync state is the source of truth,
 * and Rust handles reconnection internally.
 *
 * @example
 * ```typescript
 * function MyCalendar({ calendarId }) {
 *   const { connectionState } = useTauriSse({
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
export function useTauriSse(config: UseTauriSseConfig): UseTauriSseResult {
  const {
    calendarId,
    enabled = true,
    onEntryAdded,
    onEntryUpdated,
    onEntryDeleted,
    onConnectionChange,
    onError,
  } = config

  const unlistenersRef = useRef<Array<() => void>>([])

  // Use shared connection manager for reactive connection state
  const { connectionState, updateConnectionState } = useConnectionManager({
    onConnectionChange,
  })

  // Use shared Dexie handlers for SSE event processing
  const {
    handleEntryAdded: dexieHandleAdded,
    handleEntryUpdated: dexieHandleUpdated,
    handleEntryDeleted: dexieHandleDeleted,
  } = useDexieHandlers()

  // Store entry callbacks in refs (connection callback handled by useConnectionManager)
  const entryCallbacksRef = useRef({ onEntryAdded, onEntryUpdated, onEntryDeleted, onError })
  useEffect(() => {
    entryCallbacksRef.current = { onEntryAdded, onEntryUpdated, onEntryDeleted, onError }
  }, [onEntryAdded, onEntryUpdated, onEntryDeleted, onError])

  /**
   * Handle entry_added event: update Dexie via shared handler, then call callback.
   */
  const handleEntryAdded = useCallback(
    async (entry: ServerEntry, date: string) => {
      await dexieHandleAdded(entry)
      entryCallbacksRef.current.onEntryAdded?.(entry, date)
    },
    [dexieHandleAdded],
  )

  /**
   * Handle entry_updated event: update Dexie via shared handler, then call callback.
   */
  const handleEntryUpdated = useCallback(
    async (entry: ServerEntry, date: string) => {
      await dexieHandleUpdated(entry)
      entryCallbacksRef.current.onEntryUpdated?.(entry, date)
    },
    [dexieHandleUpdated],
  )

  /**
   * Handle entry_deleted event: remove from Dexie via shared handler, then call callback.
   */
  const handleEntryDeleted = useCallback(
    async (entryId: string, date: string) => {
      await dexieHandleDeleted(entryId)
      entryCallbacksRef.current.onEntryDeleted?.(entryId, date)
    },
    [dexieHandleDeleted],
  )

  /**
   * Reconnect to SSE (manual trigger).
   */
  const reconnect = useCallback(async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core")
      await invoke("stop_sse")
      // Pass null to let Rust use its tracked lastEventId
      await invoke("start_sse", { calendarId, lastEventId: null })
    } catch (e) {
      const error = e instanceof Error ? e : new Error(String(e))
      entryCallbacksRef.current.onError?.(error, "reconnect_sse")
    }
  }, [calendarId])

  /**
   * Disconnect from SSE (manual trigger).
   */
  const disconnect = useCallback(async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core")
      await invoke("stop_sse")
    } catch (e) {
      const error = e instanceof Error ? e : new Error(String(e))
      entryCallbacksRef.current.onError?.(error, "disconnect_sse")
    }
  }, [])

  // Load Tauri APIs and connect
  useEffect(() => {
    if (!enabled) return

    let mounted = true

    const init = async () => {
      // Dynamic import of Tauri APIs
      const [{ invoke }, { listen }] = await Promise.all([
        import("@tauri-apps/api/core"),
        import("@tauri-apps/api/event"),
      ])

      if (!mounted) return

      // Set up event listeners
      const unlisteners: Array<() => void> = []

      // Connection state events - use updateConnectionState for reactive state
      unlisteners.push(
        await listen<SseConnectionState>("sse:connection_state", (event) => {
          updateConnectionState(event.payload)
        }),
      )

      // Entry added events
      unlisteners.push(
        await listen<{ data: { entry: ServerEntry; date: string } }>("sse:entry_added", (event) => {
          const { entry, date } = event.payload.data
          handleEntryAdded(entry, date).catch((e) => {
            const error = e instanceof Error ? e : new Error(String(e))
            entryCallbacksRef.current.onError?.(error, "handle_entry_added")
          })
        }),
      )

      // Entry updated events
      unlisteners.push(
        await listen<{ data: { entry: ServerEntry; date: string } }>(
          "sse:entry_updated",
          (event) => {
            const { entry, date } = event.payload.data
            handleEntryUpdated(entry, date).catch((e) => {
              const error = e instanceof Error ? e : new Error(String(e))
              entryCallbacksRef.current.onError?.(error, "handle_entry_updated")
            })
          },
        ),
      )

      // Entry deleted events
      unlisteners.push(
        await listen<{ data: { entry_id: string; date: string } }>("sse:entry_deleted", (event) => {
          const { entry_id, date } = event.payload.data
          handleEntryDeleted(entry_id, date).catch((e) => {
            const error = e instanceof Error ? e : new Error(String(e))
            entryCallbacksRef.current.onError?.(error, "handle_entry_deleted")
          })
        }),
      )

      unlistenersRef.current = unlisteners

      // Start SSE connection
      try {
        await invoke("start_sse", { calendarId, lastEventId: null })
      } catch (e) {
        const error = e instanceof Error ? e : new Error(String(e))
        entryCallbacksRef.current.onError?.(error, "start_sse")
        updateConnectionState("error")
      }
    }

    init()

    return () => {
      mounted = false

      // Clean up listeners
      for (const unlisten of unlistenersRef.current) {
        unlisten()
      }
      unlistenersRef.current = []

      // Stop SSE
      import("@tauri-apps/api/core")
        .then(({ invoke }) => invoke("stop_sse"))
        .catch((e) => console.error("Failed to stop SSE:", e))
    }
  }, [
    calendarId,
    enabled,
    handleEntryAdded,
    handleEntryUpdated,
    handleEntryDeleted,
    updateConnectionState,
  ])

  return {
    connectionState,
    reconnect,
    disconnect,
    // lastEventId is tracked in the Rust backend.
    // Dexie sync state is the source of truth for the frontend,
    // and Rust handles reconnection internally.
    lastEventId: null,
  }
}
