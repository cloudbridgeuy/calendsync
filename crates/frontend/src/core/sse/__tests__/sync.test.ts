import { describe, expect, test } from "bun:test"
import type { PendingOperationType } from "@core/sync/types"
import { determineSyncAction, determineUpdateSyncAction } from "../sync"

interface TestEntry {
  pendingOperation: PendingOperationType | null
}

describe("determineSyncAction", () => {
  test("returns 'confirm_create' when pendingOperation is 'create'", () => {
    const existing: TestEntry = { pendingOperation: "create" }
    expect(determineSyncAction(existing)).toBe("confirm_create")
  })

  test("returns 'add_new' when pendingOperation is null", () => {
    const existing: TestEntry = { pendingOperation: null }
    expect(determineSyncAction(existing)).toBe("add_new")
  })

  test("returns 'add_new' when existing is undefined", () => {
    expect(determineSyncAction(undefined)).toBe("add_new")
  })

  test("returns 'add_new' when pendingOperation is 'update'", () => {
    const existing: TestEntry = { pendingOperation: "update" }
    expect(determineSyncAction(existing)).toBe("add_new")
  })

  test("returns 'add_new' when pendingOperation is 'delete'", () => {
    const existing: TestEntry = { pendingOperation: "delete" }
    expect(determineSyncAction(existing)).toBe("add_new")
  })
})

describe("determineUpdateSyncAction", () => {
  test("returns 'confirm_update' when pendingOperation is 'update'", () => {
    const existing: TestEntry = { pendingOperation: "update" }
    expect(determineUpdateSyncAction(existing)).toBe("confirm_update")
  })

  test("returns 'apply_remote' when pendingOperation is null", () => {
    const existing: TestEntry = { pendingOperation: null }
    expect(determineUpdateSyncAction(existing)).toBe("apply_remote")
  })

  test("returns 'apply_remote' when existing is undefined", () => {
    expect(determineUpdateSyncAction(undefined)).toBe("apply_remote")
  })

  test("returns 'apply_remote' when pendingOperation is 'create'", () => {
    const existing: TestEntry = { pendingOperation: "create" }
    expect(determineUpdateSyncAction(existing)).toBe("apply_remote")
  })

  test("returns 'apply_remote' when pendingOperation is 'delete'", () => {
    const existing: TestEntry = { pendingOperation: "delete" }
    expect(determineUpdateSyncAction(existing)).toBe("apply_remote")
  })
})
