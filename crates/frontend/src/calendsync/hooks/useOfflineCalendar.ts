/**
 * Offline-first calendar hook using Dexie for local storage.
 *
 * This hook provides:
 * - Reactive queries via useLiveQuery from Dexie
 * - Local-first writes with pending sync queue
 * - Entries grouped by date using pure functions
 *
 * Data flow:
 * 1. User creates/updates/deletes entry
 * 2. Entry written to Dexie with syncStatus: "pending"
 * 3. SyncEngine queues a PendingOperation
 * 4. When online, SyncEngine processes queue via HTTP API
 * 5. SSE confirms sync, entry marked "synced"
 */

import { groupEntriesByDate } from "@core/calendar/entries"
import type { ServerEntry } from "@core/calendar/types"
import { formDataToEntry } from "@core/sync/transformations"
import type { LocalEntry } from "@core/sync/types"
import { useLiveQuery } from "dexie-react-hooks"
import { useCallback, useMemo } from "react"
import { db } from "../db"
import type { EntryFormData } from "../types"
import { useSyncEngine } from "./useSyncEngine"

/**
 * Configuration for useOfflineCalendar hook.
 */
export interface UseOfflineCalendarConfig {
  /** Calendar ID for filtering entries */
  calendarId: string
}

/**
 * Result from useOfflineCalendar hook.
 */
export interface UseOfflineCalendarResult {
  /** All entries for this calendar */
  entries: LocalEntry[] | undefined
  /** Entries grouped by date (YYYY-MM-DD -> entries[]) */
  entriesByDate: Map<string, LocalEntry[]>
  /** Create a new entry locally */
  createEntry: (data: EntryFormData) => Promise<LocalEntry>
  /** Update an existing entry locally */
  updateEntry: (entryId: string, data: EntryFormData) => Promise<void>
  /** Delete an entry locally */
  deleteEntry: (entryId: string) => Promise<void>
  /** Toggle a task's completed status locally */
  toggleEntry: (entryId: string) => Promise<void>
  /** Whether the browser is currently online */
  isOnline: boolean
  /** Number of pending operations waiting to sync */
  pendingCount: number
  /** Whether sync is currently in progress */
  isSyncing: boolean
}

/**
 * Hook for offline-first calendar operations.
 *
 * Uses Dexie's useLiveQuery for reactive queries that automatically
 * update when the underlying IndexedDB data changes.
 *
 * @example
 * ```typescript
 * function MyCalendar({ calendarId }) {
 *   const {
 *     entries,
 *     entriesByDate,
 *     createEntry,
 *     isOnline,
 *     pendingCount,
 *   } = useOfflineCalendar({ calendarId })
 *
 *   const handleCreate = async (data) => {
 *     await createEntry(data)
 *     // Entry is immediately available locally
 *     // Will sync to server when online
 *   }
 *
 *   return (
 *     <div>
 *       {!isOnline && <span>Offline mode</span>}
 *       {pendingCount > 0 && <span>{pendingCount} pending</span>}
 *       {Array.from(entriesByDate.entries()).map(([date, dayEntries]) => (
 *         <Day key={date} date={date} entries={dayEntries} />
 *       ))}
 *     </div>
 *   )
 * }
 * ```
 */
export function useOfflineCalendar(config: UseOfflineCalendarConfig): UseOfflineCalendarResult {
  const { calendarId } = config
  const { isOnline, isSyncing, pendingCount, queueOperation } = useSyncEngine()

  // Reactive query for all entries in this calendar
  const entries = useLiveQuery(
    () => db.entries.where("calendarId").equals(calendarId).toArray(),
    [calendarId],
  )

  // Group entries by date using pure function
  const entriesByDate = useMemo(() => {
    if (!entries) return new Map<string, LocalEntry[]>()
    // groupEntriesByDate is generic and preserves LocalEntry type
    return groupEntriesByDate(entries)
  }, [entries])

  /**
   * Create a new entry locally and queue for sync.
   */
  const createEntry = useCallback(
    async (data: EntryFormData): Promise<LocalEntry> => {
      const tempId = crypto.randomUUID()
      const now = new Date().toISOString()

      const entryData = formDataToEntry(data, calendarId)

      const entry: LocalEntry = {
        ...entryData,
        id: tempId,
        syncStatus: "pending",
        localUpdatedAt: now,
        pendingOperation: "create",
      }

      // Write to local database
      await db.entries.add(entry)

      // Queue operation for sync
      await queueOperation(tempId, "create", entry as Partial<ServerEntry>)

      return entry
    },
    [calendarId, queueOperation],
  )

  /**
   * Update an existing entry locally and queue for sync.
   */
  const updateEntry = useCallback(
    async (entryId: string, data: EntryFormData): Promise<void> => {
      const existing = await db.entries.get(entryId)
      if (!existing) {
        throw new Error(`Entry not found: ${entryId}`)
      }

      const now = new Date().toISOString()
      const entryData = formDataToEntry(data, calendarId, existing)

      const updated: LocalEntry = {
        ...existing,
        ...entryData,
        syncStatus: "pending",
        localUpdatedAt: now,
        pendingOperation: "update",
      }

      // Write to local database
      await db.entries.put(updated)

      // Queue operation for sync
      await queueOperation(entryId, "update", updated as Partial<ServerEntry>)
    },
    [calendarId, queueOperation],
  )

  /**
   * Delete an entry locally and queue for sync.
   */
  const deleteEntry = useCallback(
    async (entryId: string): Promise<void> => {
      const existing = await db.entries.get(entryId)
      if (!existing) {
        // Entry doesn't exist locally, nothing to delete
        return
      }

      // Always clear ALL pending operations for this entry first.
      // This prevents orphaned create/update operations when deleting an entry
      // that was created locally and then updated before being synced.
      await db.pending_operations.where("entryId").equals(entryId).delete()

      // If entry was created locally but never synced, just delete it
      if (existing.pendingOperation === "create") {
        await db.entries.delete(entryId)
        return
      }

      // Mark as pending delete in local database
      const now = new Date().toISOString()
      const updated: LocalEntry = {
        ...existing,
        syncStatus: "pending",
        localUpdatedAt: now,
        pendingOperation: "delete",
      }
      await db.entries.put(updated)

      // Queue operation for sync
      await queueOperation(entryId, "delete", null)
    },
    [queueOperation],
  )

  /**
   * Toggle a task's completed status locally and queue for sync.
   */
  const toggleEntry = useCallback(
    async (entryId: string): Promise<void> => {
      const existing = await db.entries.get(entryId)
      if (!existing) {
        throw new Error(`Entry not found: ${entryId}`)
      }

      if (!existing.isTask) {
        throw new Error(`Entry is not a task: ${entryId}`)
      }

      const now = new Date().toISOString()
      const updated: LocalEntry = {
        ...existing,
        completed: !existing.completed,
        syncStatus: "pending",
        localUpdatedAt: now,
        pendingOperation: "update",
      }

      // Write to local database
      await db.entries.put(updated)

      // Queue operation for sync
      await queueOperation(entryId, "update", updated as Partial<ServerEntry>)
    },
    [queueOperation],
  )

  return {
    entries,
    entriesByDate,
    createEntry,
    updateEntry,
    deleteEntry,
    toggleEntry,
    isOnline,
    pendingCount,
    isSyncing,
  }
}
