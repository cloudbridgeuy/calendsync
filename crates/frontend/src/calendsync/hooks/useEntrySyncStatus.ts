import type { SyncStatus } from "@core/sync/types"
import { useCalendarContext } from "../contexts/CalendarContext"

/**
 * Get sync status for an entry from local storage.
 *
 * @param entryId - The ID of the entry
 * @returns The sync status if offline mode is enabled, undefined otherwise
 */
export function useEntrySyncStatus(entryId: string): SyncStatus | undefined {
  const { offlineEnabled, getLocalEntry } = useCalendarContext()
  if (!offlineEnabled) return undefined
  return getLocalEntry(entryId)?.syncStatus
}
