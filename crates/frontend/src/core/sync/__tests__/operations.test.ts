import { describe, expect, test } from "bun:test"
import {
  createPendingOperation,
  incrementRetry,
  markAsConflict,
  markAsSynced,
  setOperationError,
  shouldRetry,
  sortByCreatedAt,
} from "../operations"
import type { LocalEntry, PendingOperation } from "../types"

function createTestEntry(overrides: Partial<LocalEntry> = {}): LocalEntry {
  return {
    id: "test-id",
    calendarId: "cal-id",
    kind: "all-day",
    completed: false,
    isMultiDay: false,
    isAllDay: true,
    isTimed: false,
    isTask: false,
    title: "Test Entry",
    description: null,
    location: null,
    color: null,
    startDate: "2024-01-15",
    endDate: "2024-01-15",
    startTime: null,
    endTime: null,
    syncStatus: "synced",
    localUpdatedAt: "2024-01-15T10:00:00.000Z",
    pendingOperation: null,
    ...overrides,
  }
}

function createTestOperation(overrides: Partial<PendingOperation> = {}): PendingOperation {
  return {
    id: "op-id",
    entryId: "entry-id",
    operation: "create",
    payload: null,
    createdAt: "2024-01-15T10:00:00.000Z",
    retryCount: 0,
    lastError: null,
    ...overrides,
  }
}

describe("shouldRetry", () => {
  test("returns true when retry_count < maxRetries", () => {
    const op = createTestOperation({ retryCount: 2 })
    expect(shouldRetry(op, 3)).toBe(true)
  })

  test("returns true when retry_count is 0", () => {
    const op = createTestOperation({ retryCount: 0 })
    expect(shouldRetry(op, 3)).toBe(true)
  })

  test("returns false when retry_count >= maxRetries", () => {
    const op = createTestOperation({ retryCount: 3 })
    expect(shouldRetry(op, 3)).toBe(false)
  })

  test("returns false when retry_count > maxRetries", () => {
    const op = createTestOperation({ retryCount: 5 })
    expect(shouldRetry(op, 3)).toBe(false)
  })
})

describe("incrementRetry", () => {
  test("increases retry_count by 1", () => {
    const op = createTestOperation({ retryCount: 2 })
    const result = incrementRetry(op)
    expect(result.retryCount).toBe(3)
  })

  test("returns a new object (immutable)", () => {
    const op = createTestOperation({ retryCount: 0 })
    const result = incrementRetry(op)
    expect(result).not.toBe(op)
    expect(op.retryCount).toBe(0)
  })

  test("preserves other properties", () => {
    const op = createTestOperation({
      id: "custom-id",
      entryId: "custom-entry",
      operation: "update",
      retryCount: 1,
    })
    const result = incrementRetry(op)
    expect(result.id).toBe("custom-id")
    expect(result.entryId).toBe("custom-entry")
    expect(result.operation).toBe("update")
  })
})

describe("sortByCreatedAt", () => {
  test("sorts oldest first", () => {
    const ops = [
      createTestOperation({ id: "c", createdAt: "2024-01-15T12:00:00.000Z" }),
      createTestOperation({ id: "a", createdAt: "2024-01-15T08:00:00.000Z" }),
      createTestOperation({ id: "b", createdAt: "2024-01-15T10:00:00.000Z" }),
    ]
    const result = sortByCreatedAt(ops)
    expect(result.map((op) => op.id)).toEqual(["a", "b", "c"])
  })

  test("returns a new array (immutable)", () => {
    const ops = [
      createTestOperation({ id: "b", createdAt: "2024-01-15T12:00:00.000Z" }),
      createTestOperation({ id: "a", createdAt: "2024-01-15T08:00:00.000Z" }),
    ]
    const result = sortByCreatedAt(ops)
    expect(result).not.toBe(ops)
  })

  test("handles empty array", () => {
    const result = sortByCreatedAt([])
    expect(result).toEqual([])
  })

  test("handles single element", () => {
    const ops = [createTestOperation({ id: "only" })]
    const result = sortByCreatedAt(ops)
    expect(result.length).toBe(1)
    expect(result[0].id).toBe("only")
  })
})

describe("markAsSynced", () => {
  test("sets syncStatus to synced", () => {
    const entry = createTestEntry({ syncStatus: "pending" })
    const result = markAsSynced(entry)
    expect(result.syncStatus).toBe("synced")
  })

  test("clears pendingOperation", () => {
    const entry = createTestEntry({ pendingOperation: "update" })
    const result = markAsSynced(entry)
    expect(result.pendingOperation).toBeNull()
  })

  test("clears lastSyncError", () => {
    const entry = createTestEntry({ lastSyncError: "Network error" })
    const result = markAsSynced(entry)
    expect(result.lastSyncError).toBeUndefined()
  })

  test("returns a new object (immutable)", () => {
    const entry = createTestEntry()
    const result = markAsSynced(entry)
    expect(result).not.toBe(entry)
  })
})

describe("markAsConflict", () => {
  test("sets syncStatus to conflict", () => {
    const entry = createTestEntry({ syncStatus: "pending" })
    const result = markAsConflict(entry, "Server rejected")
    expect(result.syncStatus).toBe("conflict")
  })

  test("stores the error message", () => {
    const entry = createTestEntry()
    const result = markAsConflict(entry, "Network timeout")
    expect(result.lastSyncError).toBe("Network timeout")
  })

  test("returns a new object (immutable)", () => {
    const entry = createTestEntry()
    const result = markAsConflict(entry, "Error")
    expect(result).not.toBe(entry)
  })
})

describe("createPendingOperation", () => {
  test("creates operation with given entry ID", () => {
    const result = createPendingOperation("my-entry", "create", null)
    expect(result.entryId).toBe("my-entry")
  })

  test("creates operation with given operation type", () => {
    const result = createPendingOperation("entry", "delete", null)
    expect(result.operation).toBe("delete")
  })

  test("creates operation with given payload", () => {
    const payload = { title: "Updated Title" }
    const result = createPendingOperation("entry", "update", payload)
    expect(result.payload).toEqual(payload)
  })

  test("initializes retryCount to 0", () => {
    const result = createPendingOperation("entry", "create", null)
    expect(result.retryCount).toBe(0)
  })

  test("initializes lastError to null", () => {
    const result = createPendingOperation("entry", "create", null)
    expect(result.lastError).toBeNull()
  })

  test("sets createdAt to current time", () => {
    const before = new Date().toISOString()
    const result = createPendingOperation("entry", "create", null)
    const after = new Date().toISOString()
    expect(result.createdAt >= before).toBe(true)
    expect(result.createdAt <= after).toBe(true)
  })
})

describe("setOperationError", () => {
  test("sets lastError", () => {
    const op = createTestOperation()
    const result = setOperationError(op, "Connection failed")
    expect(result.lastError).toBe("Connection failed")
  })

  test("returns a new object (immutable)", () => {
    const op = createTestOperation()
    const result = setOperationError(op, "Error")
    expect(result).not.toBe(op)
  })

  test("preserves other properties", () => {
    const op = createTestOperation({ id: "my-op", retryCount: 2 })
    const result = setOperationError(op, "Error")
    expect(result.id).toBe("my-op")
    expect(result.retryCount).toBe(2)
  })
})
