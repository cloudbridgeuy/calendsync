/**
 * SyncIndicator - displays sync status for offline-first entries.
 * Shared component used by EntryTile, AllDayEntryTile, and ScheduleTimedEntry.
 */

import type { SyncStatus } from "@core/sync/types"

interface SyncIndicatorProps {
  syncStatus: SyncStatus | undefined
  classPrefix: string
}

/**
 * Renders a sync status indicator based on the entry's sync state.
 * Returns null if the entry is synced or has no status.
 */
export function SyncIndicator({ syncStatus, classPrefix }: SyncIndicatorProps) {
  if (!syncStatus || syncStatus === "synced") return null

  if (syncStatus === "pending") {
    return (
      <div className={`${classPrefix}__sync-indicator ${classPrefix}__sync-indicator--pending`}>
        <span className="sr-only">Syncing...</span>
      </div>
    )
  }

  if (syncStatus === "conflict") {
    return (
      <div className={`${classPrefix}__sync-indicator ${classPrefix}__sync-indicator--conflict`}>
        <span className="sr-only">Sync conflict</span>
      </div>
    )
  }

  return null
}
