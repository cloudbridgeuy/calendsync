/**
 * React hook for SyncEngine.
 *
 * Provides reactive state for online status, syncing status,
 * and pending operation count. Uses SyncEngine from context.
 *
 * Must be used within a SyncEngineProvider.
 */

import type { ServerEntry } from "@core/calendar/types"
import type { PendingOperationType } from "@core/sync/types"
import { useLiveQuery } from "dexie-react-hooks"
import { useCallback, useEffect, useState } from "react"

import { useSyncEngineContext } from "../contexts"
import { db } from "../db"
import { createOperationInput } from "../sync/engine"

/** Result from useSyncEngine hook */
export interface UseSyncEngineResult {
  /** Whether the browser is currently online */
  isOnline: boolean
  /** Whether sync is currently in progress */
  isSyncing: boolean
  /** Number of pending operations waiting to sync */
  pendingCount: number
  /** Queue an operation for sync */
  queueOperation: (
    entryId: string,
    operation: PendingOperationType,
    payload: Partial<ServerEntry> | null,
  ) => Promise<void>
  /** Manually trigger sync (useful for retry) */
  syncNow: () => Promise<void>
  /** Last error from queueOperation, if any */
  lastError: string | null
}

/**
 * Hook for accessing the SyncEngine.
 *
 * Provides reactive state for:
 * - Online/offline status
 * - Syncing status
 * - Pending operation count
 *
 * And methods for:
 * - Queueing operations
 * - Manually triggering sync
 *
 * @example
 * ```typescript
 * function MyComponent() {
 *   const { isOnline, pendingCount, queueOperation } = useSyncEngine()
 *
 *   const handleSave = async (entry) => {
 *     await queueOperation(entry.id, "create", entry)
 *   }
 *
 *   return (
 *     <div>
 *       {!isOnline && <span>Offline - changes will sync later</span>}
 *       {pendingCount > 0 && <span>{pendingCount} pending</span>}
 *     </div>
 *   )
 * }
 * ```
 */
export function useSyncEngine(): UseSyncEngineResult {
  const engine = useSyncEngineContext()
  const [isOnline, setIsOnline] = useState(() =>
    typeof navigator !== "undefined" ? navigator.onLine : true,
  )
  const [isSyncing, setIsSyncing] = useState(false)
  const [lastError, setLastError] = useState<string | null>(null)

  // Use Dexie's liveQuery for reactive pending count (no polling needed)
  const pendingCount = useLiveQuery(() => db.pending_operations.count(), [], 0) ?? 0

  // Subscribe to engine state changes
  useEffect(() => {
    // Initial state
    setIsOnline(engine.getIsOnline())
    setIsSyncing(engine.getIsSyncing())

    // Subscribe to state changes (replaces 500ms polling)
    const unsubscribe = engine.addListener(() => {
      setIsOnline(engine.getIsOnline())
      setIsSyncing(engine.getIsSyncing())
    })

    return unsubscribe
  }, [engine])

  /**
   * Queue an operation for sync.
   */
  const queueOperation = useCallback(
    async (
      entryId: string,
      operation: PendingOperationType,
      payload: Partial<ServerEntry> | null,
    ): Promise<void> => {
      setLastError(null)

      try {
        const input = createOperationInput(entryId, operation, payload)
        await engine.queueOperation(input)
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error)
        setLastError(message)
        throw error
      }
    },
    [engine],
  )

  /**
   * Manually trigger sync.
   */
  const syncNow = useCallback(async (): Promise<void> => {
    setIsSyncing(true)
    try {
      await engine.syncPending()
    } finally {
      setIsSyncing(engine.getIsSyncing())
    }
  }, [engine])

  return {
    isOnline,
    isSyncing,
    pendingCount,
    queueOperation,
    syncNow,
    lastError,
  }
}
