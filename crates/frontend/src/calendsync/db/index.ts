/**
 * Dexie database for offline-first calendar entries.
 *
 * This module provides IndexedDB storage via Dexie.js for:
 * - Local entry storage with sync status tracking
 * - Pending operation queue for offline mutations
 * - Per-calendar sync state tracking
 */

import Dexie, { type EntityTable } from "dexie"
import type { LocalEntry, PendingOperation, SyncState } from "../../core/sync/types"

/**
 * CalendSync database schema.
 *
 * Tables:
 * - entries: Local calendar entries with sync tracking
 * - pending_operations: Queue of operations to sync with server
 * - sync_state: Per-calendar sync state tracking
 */
export class CalendSyncDatabase extends Dexie {
  entries!: EntityTable<LocalEntry, "id">
  pending_operations!: EntityTable<PendingOperation, "id">
  sync_state!: EntityTable<SyncState, "calendarId">

  constructor() {
    super("calendsync")

    this.version(1).stores({
      // Entries table:
      // - id: primary key
      // - calendarId: for filtering by calendar
      // - startDate: for date-based queries
      // - [calendarId+startDate]: compound index for efficient calendar+date queries
      // - syncStatus: for finding entries needing sync
      entries: "id, calendarId, startDate, [calendarId+startDate], syncStatus",

      // Pending operations table:
      // - id: primary key (auto-generated UUID)
      // - entryId: for finding operations for a specific entry
      // - createdAt: for ordering operations
      pending_operations: "id, entryId, createdAt",

      // Sync state table:
      // - calendarId: primary key (one state per calendar)
      sync_state: "calendarId",
    })
  }
}

/**
 * Singleton database instance.
 * Import this to interact with the local database.
 */
export const db = new CalendSyncDatabase()

// Re-export types for convenience
export type { LocalEntry, PendingOperation, SyncState } from "../../core/sync/types"
