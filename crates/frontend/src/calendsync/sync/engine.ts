/**
 * SyncEngine - Orchestrates offline sync operations.
 *
 * Responsibilities:
 * - Track online/offline state via browser events
 * - Queue operations to Dexie when offline
 * - Process pending operations when coming online
 * - Retry failed operations up to 3 times
 * - Mark entries as "conflict" when max retries exceeded
 *
 * This is the Imperative Shell that coordinates I/O operations,
 * using pure functions from @core/sync/operations for logic.
 */

import { entryToFormData, formDataToApiPayload } from "@core/calendar"
import type { ServerEntry } from "@core/calendar/types"
import { deriveEntryTypeFromFlags } from "@core/sse/connection"
import {
  incrementRetry,
  markAsConflict,
  setOperationError,
  shouldRetry,
  sortByCreatedAt,
} from "@core/sync/operations"
import type { PendingOperation, PendingOperationType } from "@core/sync/types"
import type { Transport } from "@core/transport/types"

import type { CalendSyncDatabase } from "../db"
import { getControlPlaneUrl } from "../hooks/useApi"

/** Maximum number of retry attempts before marking as conflict */
const MAX_RETRIES = 3

/** Result of executing an operation */
interface OperationResult {
  success: boolean
  error?: string
  /** Server entry returned on successful create/update */
  entry?: ServerEntry
}

/**
 * API client interface for sync operations.
 * Abstracted for testability.
 */
export interface SyncApiClient {
  createEntry(calendarId: string, payload: Partial<ServerEntry>): Promise<ServerEntry>
  updateEntry(entryId: string, payload: Partial<ServerEntry>): Promise<ServerEntry>
  deleteEntry(entryId: string): Promise<void>
}

/**
 * Default API client implementation using fetch.
 * Used as fallback when transport is not initialized (web before TransportProvider).
 */
function createDefaultApiClient(): SyncApiClient {
  const baseUrl = getControlPlaneUrl()

  return {
    async createEntry(calendarId: string, payload: Partial<ServerEntry>): Promise<ServerEntry> {
      const formData = payloadToFormData(payload, calendarId)

      const response = await fetch(`${baseUrl}/api/entries`, {
        method: "POST",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
        },
        body: formData.toString(),
        credentials: "include",
      })

      if (!response.ok) {
        const text = await response.text()
        throw new Error(`Failed to create entry: ${response.status} ${text}`)
      }

      return response.json()
    },

    async updateEntry(entryId: string, payload: Partial<ServerEntry>): Promise<ServerEntry> {
      const calendarId = payload.calendarId ?? ""
      const formData = payloadToFormData(payload, calendarId)

      const response = await fetch(`${baseUrl}/api/entries/${entryId}`, {
        method: "PUT",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
        },
        body: formData.toString(),
        credentials: "include",
      })

      if (!response.ok) {
        const text = await response.text()
        throw new Error(`Failed to update entry: ${response.status} ${text}`)
      }

      return response.json()
    },

    async deleteEntry(entryId: string): Promise<void> {
      const response = await fetch(`${baseUrl}/api/entries/${entryId}`, {
        method: "DELETE",
        credentials: "include",
      })

      if (!response.ok) {
        const text = await response.text()
        throw new Error(`Failed to delete entry: ${response.status} ${text}`)
      }
    },
  }
}

/**
 * Create an API client backed by the Transport layer.
 * Used by SyncEngine when transport is initialized.
 */
function createTransportApiClient(transport: Transport): SyncApiClient {
  return {
    async createEntry(calendarId: string, payload: Partial<ServerEntry>): Promise<ServerEntry> {
      const entryType = deriveEntryTypeFromFlags(payload)
      return transport.createEntry({
        calendar_id: calendarId,
        title: payload.title ?? "",
        date: payload.startDate ?? "",
        start_time: payload.startTime ?? undefined,
        end_time: payload.endTime ?? undefined,
        all_day: payload.isAllDay ?? entryType === "all_day",
        description: payload.description ?? undefined,
        entry_type: entryType,
      })
    },

    async updateEntry(entryId: string, payload: Partial<ServerEntry>): Promise<ServerEntry> {
      const calendarId = payload.calendarId ?? ""
      const entryType = deriveEntryTypeFromFlags(payload)
      return transport.updateEntry(entryId, {
        calendar_id: calendarId,
        title: payload.title ?? "",
        date: payload.startDate ?? "",
        start_time: payload.startTime ?? undefined,
        end_time: payload.endTime ?? undefined,
        all_day: payload.isAllDay ?? entryType === "all_day",
        description: payload.description ?? undefined,
        entry_type: entryType,
      })
    },

    async deleteEntry(entryId: string): Promise<void> {
      return transport.deleteEntry(entryId)
    },
  }
}

/**
 * Convert a partial ServerEntry payload to form data for API calls.
 * Uses entryToFormData pattern for complete entries, or builds manually for partials.
 */
function payloadToFormData(payload: Partial<ServerEntry>, calendarId: string): URLSearchParams {
  // If we have a complete ServerEntry, use the existing conversion
  if (isCompleteEntry(payload)) {
    const formData = entryToFormData(payload as ServerEntry)
    return formDataToApiPayload(formData, calendarId)
  }

  // For partial payloads, build form data manually
  const entryType = deriveEntryTypeFromFlags(payload)
  const formData = {
    title: payload.title ?? "",
    startDate: payload.startDate ?? "",
    endDate: payload.endDate,
    isAllDay: payload.isAllDay ?? entryType === "all_day",
    description: payload.description ?? undefined,
    location: payload.location ?? undefined,
    entryType,
    startTime: payload.startTime ?? undefined,
    endTime: payload.endTime ?? undefined,
    completed: payload.completed,
  }

  return formDataToApiPayload(formData, calendarId)
}

/**
 * Check if a partial ServerEntry has enough fields to be considered complete.
 */
function isCompleteEntry(payload: Partial<ServerEntry>): boolean {
  return (
    typeof payload.id === "string" &&
    typeof payload.title === "string" &&
    typeof payload.startDate === "string" &&
    typeof payload.isAllDay === "boolean"
  )
}

/**
 * SyncEngine orchestrates offline sync operations.
 *
 * Usage:
 * ```typescript
 * const engine = new SyncEngine(db)
 * await engine.queueOperation({
 *   entryId: "123",
 *   operation: "create",
 *   payload: { title: "Meeting", ... }
 * })
 * ```
 */
export class SyncEngine {
  private isOnline: boolean
  private isSyncing: boolean = false
  private pendingWhileSyncing: boolean = false
  private api: SyncApiClient
  private transport: Transport | null = null
  private onlineHandler: () => void
  private offlineHandler: () => void
  private listeners: Set<() => void> = new Set()

  constructor(
    private db: CalendSyncDatabase,
    api?: SyncApiClient,
  ) {
    // Use provided API client or create default
    this.api = api ?? createDefaultApiClient()

    // Initialize online state from browser
    this.isOnline = typeof navigator !== "undefined" ? navigator.onLine : true

    // Bind event handlers for cleanup
    this.onlineHandler = () => this.handleOnline()
    this.offlineHandler = () => this.handleOffline()

    // Register event listeners
    if (typeof window !== "undefined") {
      window.addEventListener("online", this.onlineHandler)
      window.addEventListener("offline", this.offlineHandler)
    }
  }

  /**
   * Initialize transport for API calls.
   * Must be called from within TransportProvider context.
   * This allows the singleton SyncEngine to use the transport layer
   * for cross-platform compatibility (web + Tauri).
   */
  initTransport(transport: Transport): void {
    if (this.transport === transport) return // Already initialized with same transport
    this.transport = transport
    this.api = createTransportApiClient(transport)
  }

  /**
   * Check if transport has been initialized.
   */
  hasTransport(): boolean {
    return this.transport !== null
  }

  /**
   * Clean up event listeners.
   * Call this when the engine is no longer needed.
   */
  dispose(): void {
    if (typeof window !== "undefined") {
      window.removeEventListener("online", this.onlineHandler)
      window.removeEventListener("offline", this.offlineHandler)
    }
  }

  /**
   * Get current online status.
   */
  getIsOnline(): boolean {
    return this.isOnline
  }

  /**
   * Get current syncing status.
   */
  getIsSyncing(): boolean {
    return this.isSyncing
  }

  /** Subscribe to state changes. Returns unsubscribe function. */
  addListener(callback: () => void): () => void {
    this.listeners.add(callback)
    return () => this.listeners.delete(callback)
  }

  private notifyListeners(): void {
    for (const listener of this.listeners) {
      listener()
    }
  }

  /**
   * Get count of pending operations.
   */
  async getPendingCount(): Promise<number> {
    return this.db.pending_operations.count()
  }

  /**
   * Queue an operation for sync.
   *
   * The operation is immediately stored in Dexie. If online,
   * sync is triggered automatically.
   *
   * @param op - Operation to queue (id and created_at are auto-generated)
   */
  async queueOperation(
    op: Omit<PendingOperation, "id" | "createdAt" | "retryCount" | "lastError">,
  ): Promise<void> {
    const pending: PendingOperation = {
      ...op,
      id: crypto.randomUUID(),
      createdAt: new Date().toISOString(),
      retryCount: 0,
      lastError: null,
    }

    await this.db.pending_operations.add(pending)

    if (this.isOnline) {
      // Trigger sync but don't await - let it run in background
      this.syncPending()
    }
  }

  /**
   * Handle coming online.
   * Triggers sync of pending operations.
   */
  private handleOnline(): void {
    this.isOnline = true
    this.notifyListeners()
    this.syncPending()
  }

  /**
   * Handle going offline.
   */
  private handleOffline(): void {
    this.isOnline = false
    this.notifyListeners()
  }

  /**
   * Process all pending operations.
   *
   * Operations are processed in order (oldest first).
   * Failed operations are retried up to MAX_RETRIES times.
   * After max retries, entries are marked as "conflict".
   */
  async syncPending(): Promise<void> {
    // Prevent concurrent sync - mark if work arrives during sync
    if (this.isSyncing) {
      this.pendingWhileSyncing = true
      return
    }
    this.isSyncing = true
    this.notifyListeners()

    try {
      const pending = await this.db.pending_operations.toArray()
      const sorted = sortByCreatedAt(pending)

      for (const op of sorted) {
        // Check if still online before each operation
        if (!this.isOnline) break

        const result = await this.executeOperation(op)

        if (result.success) {
          // Operation succeeded - remove from queue
          await this.db.pending_operations.delete(op.id)

          // Update local entry if we got a server response
          if (result.entry && op.operation !== "delete") {
            await this.db.entries.update(op.entryId, {
              syncStatus: "synced",
              pendingOperation: null,
              lastSyncError: undefined,
            })
          } else if (op.operation === "delete") {
            // For deletes, remove the local entry if it exists
            await this.db.entries.delete(op.entryId)
          }
        } else if (shouldRetry(op, MAX_RETRIES)) {
          // Retry later - increment retry count and store error
          const updatedOp = incrementRetry(setOperationError(op, result.error ?? "Unknown error"))
          await this.db.pending_operations.put(updatedOp)
        } else {
          // Max retries exceeded - mark as conflict
          const entry = await this.db.entries.get(op.entryId)
          if (entry) {
            const conflictEntry = markAsConflict(entry, result.error ?? "Max retries exceeded")
            await this.db.entries.put(conflictEntry)
          }

          // Remove from pending queue
          await this.db.pending_operations.delete(op.id)
        }
      }
    } finally {
      this.isSyncing = false
      this.notifyListeners()
      // If new operations arrived during sync, re-run sync
      if (this.pendingWhileSyncing) {
        this.pendingWhileSyncing = false
        await this.syncPending()
      }
    }
  }

  /**
   * Execute a single operation against the API.
   *
   * @param op - The pending operation to execute
   * @returns Result indicating success/failure
   */
  private async executeOperation(op: PendingOperation): Promise<OperationResult> {
    try {
      switch (op.operation) {
        case "create": {
          if (!op.payload) {
            return { success: false, error: "Create operation requires payload" }
          }
          const calendarId = op.payload.calendarId ?? ""
          const entry = await this.api.createEntry(calendarId, op.payload)
          return { success: true, entry }
        }

        case "update": {
          if (!op.payload) {
            return { success: false, error: "Update operation requires payload" }
          }
          const entry = await this.api.updateEntry(op.entryId, op.payload)
          return { success: true, entry }
        }

        case "delete": {
          await this.api.deleteEntry(op.entryId)
          return { success: true }
        }

        default: {
          // TypeScript exhaustiveness check
          const _exhaustive: never = op.operation
          return { success: false, error: `Unknown operation type: ${_exhaustive}` }
        }
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      return { success: false, error: message }
    }
  }
}

/**
 * Convenience function to create a pending operation input.
 * Use this with SyncEngine.queueOperation().
 */
export function createOperationInput(
  entryId: string,
  operation: PendingOperationType,
  payload: Partial<ServerEntry> | null,
): Omit<PendingOperation, "id" | "createdAt" | "retryCount" | "lastError"> {
  return {
    entryId,
    operation,
    payload,
  }
}
