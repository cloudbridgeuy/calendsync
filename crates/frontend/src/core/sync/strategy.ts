/**
 * Sync strategy determination - pure functions with no side effects.
 * This is the Functional Core for deciding how to initialize calendar data.
 */

/**
 * Strategy for initial calendar data synchronization.
 * - use_local: Use existing local data (fastest, no network required)
 * - hydrate_ssr: Hydrate from SSR-provided data (first load optimization)
 * - full_sync: Fetch all data from server (cold start or cache miss)
 */
export type SyncStrategy = { type: "use_local" } | { type: "hydrate_ssr" } | { type: "full_sync" }

/**
 * Determines the appropriate sync strategy based on available data.
 *
 * Priority order:
 * 1. Local data with sync state - use it directly (offline-first)
 * 2. SSR data available - hydrate local storage from SSR
 * 3. Neither available - fetch from server
 *
 * @param hasLocalData - Whether there are entries in local storage
 * @param hasSyncState - Whether a sync state record exists for this calendar
 * @param hasSsrDays - Whether SSR-provided days are available
 * @returns The sync strategy to use
 */
export function decideSyncStrategy(
  hasLocalData: boolean,
  hasSyncState: boolean,
  hasSsrDays: boolean,
): SyncStrategy {
  if (hasLocalData && hasSyncState) {
    return { type: "use_local" }
  }
  if (hasSsrDays) {
    return { type: "hydrate_ssr" }
  }
  return { type: "full_sync" }
}
