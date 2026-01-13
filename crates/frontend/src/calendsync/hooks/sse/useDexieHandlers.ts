/**
 * Shared Dexie event handlers for SSE hooks.
 *
 * Extracts the common pattern of handling SSE events and updating Dexie:
 * - handleEntryAdded: Adds entry or confirms pending create
 * - handleEntryUpdated: Updates entry or confirms pending update
 * - handleEntryDeleted: Removes entry from database
 *
 * Used by: useSseWithOffline, useSseUnified (Tauri branch)
 */

import type { ServerEntry } from "@core/calendar/types"
import { determineSyncAction, determineUpdateSyncAction } from "@core/sse/sync"
import { serverToLocalEntry } from "@core/sync/types"
import { useCallback } from "react"

import { db } from "../../db"

/**
 * Result from the Dexie handlers hook.
 * Contains memoized handlers for each SSE event type.
 */
export interface DexieHandlers {
  /**
   * Handle entry_added SSE event.
   * Adds the entry to Dexie, or confirms a pending create operation.
   */
  handleEntryAdded: (entry: ServerEntry) => Promise<void>

  /**
   * Handle entry_updated SSE event.
   * Updates the entry in Dexie, or confirms a pending update operation.
   */
  handleEntryUpdated: (entry: ServerEntry) => Promise<void>

  /**
   * Handle entry_deleted SSE event.
   * Removes the entry from Dexie.
   */
  handleEntryDeleted: (entryId: string) => Promise<void>
}

/**
 * Hook providing Dexie handlers for SSE events.
 *
 * These handlers implement the sync confirmation logic:
 * - When an SSE event matches a pending local operation, it confirms the sync
 * - When an SSE event is from another client, it applies the change locally
 *
 * **Note on lastEventId tracking**: These handlers focus solely on Dexie entry
 * updates. Consumers are responsible for tracking `lastEventId` separately if
 * they need to resume from the last processed event after reconnection. This
 * is typically done in the SSE connection hook (e.g., useSseWithOffline) by
 * storing the event ID from each received message.
 *
 * @returns Memoized handlers for entry_added, entry_updated, entry_deleted
 *
 * @example
 * ```typescript
 * function useSseWithOffline(config) {
 *   const { handleEntryAdded, handleEntryUpdated, handleEntryDeleted } = useDexieHandlers()
 *
 *   eventSource.addEventListener("entry_added", async (e) => {
 *     const data = JSON.parse(e.data)
 *     await handleEntryAdded(data.entry)
 *     // Consumer tracks lastEventId separately:
 *     // lastEventIdRef.current = e.lastEventId
 *     config.onEntryAdded?.(data.entry, data.date)
 *   })
 * }
 * ```
 */
export function useDexieHandlers(): DexieHandlers {
  /**
   * Handle entry_added event.
   * Checks if this confirms a pending create operation, or is a new entry.
   */
  const handleEntryAdded = useCallback(async (entry: ServerEntry) => {
    const localEntry = serverToLocalEntry(entry)

    // Check if this is our own pending entry being confirmed
    const existing = await db.entries.get(entry.id)
    const action = determineSyncAction(existing)

    if (action === "confirm_create") {
      // Our create was confirmed - mark as synced
      await db.entries.update(entry.id, {
        ...localEntry,
        syncStatus: "synced",
        pendingOperation: null,
      })
    } else {
      // New entry from another client - add it
      // Using put() for idempotency: if the same event is received twice
      // (e.g., due to reconnection), the second put() safely overwrites
      // with identical data rather than failing on duplicate key.
      await db.entries.put(localEntry)
    }
  }, [])

  /**
   * Handle entry_updated event.
   * Checks if this confirms a pending update operation, or is a remote update.
   */
  const handleEntryUpdated = useCallback(async (entry: ServerEntry) => {
    const localEntry = serverToLocalEntry(entry)

    // Check if this is our own pending entry being confirmed
    const existing = await db.entries.get(entry.id)
    const action = determineUpdateSyncAction(existing)

    if (action === "confirm_update") {
      // Our update was confirmed - mark as synced
      await db.entries.update(entry.id, {
        ...localEntry,
        syncStatus: "synced",
        pendingOperation: null,
      })
    } else {
      // Update from another client - apply it
      await db.entries.put(localEntry)
    }
  }, [])

  /**
   * Handle entry_deleted event.
   * Removes the entry from local database.
   */
  const handleEntryDeleted = useCallback(async (entryId: string) => {
    await db.entries.delete(entryId)
  }, [])

  return {
    handleEntryAdded,
    handleEntryUpdated,
    handleEntryDeleted,
  }
}
