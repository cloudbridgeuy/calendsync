/**
 * SSE sync confirmation logic - pure functions with no side effects.
 * This is the Functional Core for determining how to handle SSE events
 * when they arrive for entries that may have pending local operations.
 */

import type { PendingOperationType } from "@core/sync/types"

/**
 * Action to take when an entry_added SSE event is received.
 * - confirm_create: The event confirms our pending create operation
 * - add_new: The event is a new entry from another client
 */
export type SyncAction = "confirm_create" | "add_new"

/**
 * Minimal interface for checking pending operation state.
 * Uses the proper PendingOperationType from sync types.
 */
interface HasPendingOperation {
  pendingOperation: PendingOperationType | null
}

/**
 * Determines what action to take when an entry_added event is received.
 *
 * If the entry exists locally with a pending create operation,
 * the SSE event confirms that our optimistic create was successful.
 * Otherwise, it's a new entry from another client that should be added.
 *
 * @param existing - The existing local entry, if any
 * @returns The action to take
 */
export function determineSyncAction(existing: HasPendingOperation | undefined): SyncAction {
  return existing?.pendingOperation === "create" ? "confirm_create" : "add_new"
}

/**
 * Action to take when an entry_updated SSE event is received.
 * - confirm_update: The event confirms our pending update operation
 * - apply_remote: The event is an update from another client
 */
export type UpdateSyncAction = "confirm_update" | "apply_remote"

/**
 * Determines what action to take when an entry_updated event is received.
 *
 * If the entry exists locally with a pending update operation,
 * the SSE event confirms that our optimistic update was successful.
 * Otherwise, it's an update from another client that should be applied.
 *
 * @param existing - The existing local entry, if any
 * @returns The action to take
 */
export function determineUpdateSyncAction(
  existing: HasPendingOperation | undefined,
): UpdateSyncAction {
  return existing?.pendingOperation === "update" ? "confirm_update" : "apply_remote"
}
