/**
 * Initial sync hook for offline-first calendar initialization.
 *
 * This hook handles:
 * - Requesting persistent storage from the browser
 * - Checking for existing local data
 * - Performing full sync if needed
 * - Returning ready state for UI rendering
 *
 * Should be used at the top level of the calendar component
 * to ensure data is available before rendering.
 */

import type { ServerDay } from "@core/calendar/types"
import { decideSyncStrategy } from "@core/sync/strategy"
import { serverToLocalEntry } from "@core/sync/types"
import { useTransport } from "@core/transport"
import type { Transport } from "@core/transport/types"
import { useCallback, useEffect, useRef, useState } from "react"

import { db } from "../db"

/**
 * Configuration for useInitialSync hook.
 */
export interface UseInitialSyncConfig {
  /** Calendar ID to sync */
  calendarId: string
  /** Initial days from SSR (if available) */
  initialDays?: ServerDay[]
  /** Number of days to fetch before and after today */
  bufferDays?: number
}

/**
 * Result from useInitialSync hook.
 */
export interface UseInitialSyncResult {
  /** Whether initial sync is complete and UI can render */
  isReady: boolean
  /** Error message if sync failed */
  error: string | null
  /** Whether sync is currently in progress */
  isSyncing: boolean
  /** Manually trigger a full sync */
  forceSync: () => Promise<void>
}

/**
 * Get today's date as YYYY-MM-DD string.
 */
function getTodayDateKey(): string {
  const today = new Date()
  const year = today.getFullYear()
  const month = String(today.getMonth() + 1).padStart(2, "0")
  const day = String(today.getDate()).padStart(2, "0")
  return `${year}-${month}-${day}`
}

/**
 * Perform a full sync from the server.
 * Fetches all entries for the calendar and stores them locally.
 */
async function performFullSync(
  transport: Transport,
  calendarId: string,
  highlightedDay: string,
  bufferDays: number,
): Promise<void> {
  // Fetch entries from server via transport
  const days = await transport.fetchEntries({
    calendarId,
    highlightedDay,
    before: bufferDays,
    after: bufferDays,
  })

  // Store all entries locally
  await db.transaction("rw", [db.entries, db.sync_state], async () => {
    // Clear existing entries for this calendar
    await db.entries.where("calendarId").equals(calendarId).delete()

    // Add all fetched entries
    for (const day of days) {
      for (const entry of day.entries) {
        const localEntry = serverToLocalEntry(entry)
        await db.entries.add(localEntry)
      }
    }

    // Update sync state
    await db.sync_state.put({
      calendarId,
      lastEventId: null,
      lastFullSync: new Date().toISOString(),
    })
  })
}

/**
 * Hydrate local database from SSR initial data.
 * Only used on first load to avoid unnecessary network requests.
 */
async function hydrateFromSsr(calendarId: string, days: ServerDay[]): Promise<void> {
  await db.transaction("rw", [db.entries, db.sync_state], async () => {
    // Add SSR entries if not already present
    for (const day of days) {
      for (const entry of day.entries) {
        const existing = await db.entries.get(entry.id)
        if (!existing) {
          const localEntry = serverToLocalEntry(entry)
          await db.entries.add(localEntry)
        }
      }
    }

    // Initialize sync state if not present
    const existingSyncState = await db.sync_state.get(calendarId)
    if (!existingSyncState) {
      await db.sync_state.put({
        calendarId,
        lastEventId: null,
        lastFullSync: new Date().toISOString(),
      })
    }
  })
}

/**
 * Hook for initializing offline-first calendar sync.
 *
 * This hook should be called at the top level of the calendar component.
 * It ensures that:
 * 1. Persistent storage is requested (if available)
 * 2. Local data exists or is fetched from server
 * 3. UI is ready to render
 *
 * @example
 * ```typescript
 * function Calendar({ calendarId, initialDays }) {
 *   const { isReady, error, isSyncing } = useInitialSync({
 *     calendarId,
 *     initialDays, // From SSR
 *   })
 *
 *   if (!isReady) {
 *     return <LoadingSpinner />
 *   }
 *
 *   if (error) {
 *     return <ErrorMessage message={error} />
 *   }
 *
 *   return <CalendarView />
 * }
 * ```
 */
export function useInitialSync(config: UseInitialSyncConfig): UseInitialSyncResult {
  const { calendarId, initialDays, bufferDays = 7 } = config
  const transport = useTransport()

  const [isReady, setIsReady] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [isSyncing, setIsSyncing] = useState(false)

  // Track component mount state to prevent updates after unmount
  const mountedRef = useRef(true)

  /**
   * Force a full sync from the server.
   * Respects component unmount to prevent state updates on unmounted component.
   */
  const forceSync = useCallback(async (): Promise<void> => {
    if (!mountedRef.current) return

    setIsSyncing(true)
    setError(null)

    try {
      const highlightedDay = getTodayDateKey()
      await performFullSync(transport, calendarId, highlightedDay, bufferDays)
      if (mountedRef.current) {
        setIsReady(true)
      }
    } catch (err) {
      if (mountedRef.current) {
        const message = err instanceof Error ? err.message : String(err)
        setError(message)
      }
      throw err
    } finally {
      if (mountedRef.current) {
        setIsSyncing(false)
      }
    }
  }, [transport, calendarId, bufferDays])

  useEffect(() => {
    let cancelled = false

    async function initialize() {
      try {
        // Request persistent storage if available
        if (typeof navigator !== "undefined" && navigator.storage?.persist) {
          try {
            const persisted = await navigator.storage.persist()
            if (!persisted) {
              console.warn("Persistent storage request was denied")
            }
          } catch (err) {
            // Non-critical error, continue initialization
            console.warn("Failed to request persistent storage:", err)
          }
        }

        // Check existing local data
        const syncState = await db.sync_state.get(calendarId)
        const localEntryCount = await db.entries.where("calendarId").equals(calendarId).count()

        // Determine sync strategy using pure function
        const hasLocalData = localEntryCount > 0
        const hasSyncState = syncState !== null
        const hasSsrDays = initialDays !== undefined && initialDays.length > 0
        const strategy = decideSyncStrategy(hasLocalData, hasSyncState, hasSsrDays)

        if (strategy.type === "use_local") {
          // We have local data - ready to render
          if (!cancelled) {
            setIsReady(true)
          }
        } else if (strategy.type === "hydrate_ssr") {
          // Hydrate from SSR data
          await hydrateFromSsr(calendarId, initialDays!)
          if (!cancelled) {
            setIsReady(true)
          }
        } else {
          // full_sync: No local data and no SSR data - fetch from server
          if (!cancelled) {
            setIsSyncing(true)
          }

          const highlightedDay = getTodayDateKey()
          await performFullSync(transport, calendarId, highlightedDay, bufferDays)

          if (!cancelled) {
            setIsSyncing(false)
            setIsReady(true)
          }
        }
      } catch (err) {
        if (!cancelled) {
          const message = err instanceof Error ? err.message : String(err)
          setError(message)
          setIsSyncing(false)
          // Still mark as ready so error can be displayed
          setIsReady(true)
        }
      }
    }

    initialize()

    return () => {
      cancelled = true
      mountedRef.current = false
    }
  }, [calendarId, initialDays, bufferDays, transport])

  return {
    isReady,
    error,
    isSyncing,
    forceSync,
  }
}
