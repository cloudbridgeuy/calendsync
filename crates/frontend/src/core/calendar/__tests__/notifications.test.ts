import { describe, expect, test } from "bun:test"
import type { Notification } from "@calendsync/types"
import { addNotification, createNotification } from "../notifications"

describe("createNotification", () => {
  test("generates deterministic ID from type and entryId", () => {
    const n = createNotification("added", "entry-123", "Lunch", "2026-03-04")
    expect(n.id).toBe("notif-added-entry-123")
  })

  test("same type+entryId produces same ID", () => {
    const a = createNotification("added", "e1", "A", "2026-01-01")
    const b = createNotification("added", "e1", "B", "2026-02-01")
    expect(a.id).toBe(b.id)
  })

  test("different type produces different ID", () => {
    const a = createNotification("added", "e1", "A", "2026-01-01")
    const b = createNotification("deleted", "e1", "A", "2026-01-01")
    expect(a.id).not.toBe(b.id)
  })
})

describe("addNotification", () => {
  const makeNotif = (id: string): Notification => ({
    id,
    type: "added",
    entryId: "e1",
    entryTitle: "Test",
    date: "2026-03-04",
    timestamp: Date.now(),
    read: false,
  })

  test("adds notification to empty list", () => {
    const n = makeNotif("notif-added-e1")
    const result = addNotification([], n)
    expect(result).toHaveLength(1)
    expect(result[0].id).toBe("notif-added-e1")
  })

  test("skips duplicate ID (idempotent)", () => {
    const existing = makeNotif("notif-added-e1")
    const duplicate = makeNotif("notif-added-e1")
    const list = [existing]
    const result = addNotification(list, duplicate)
    expect(result).toBe(list) // same reference — no-op
    expect(result).toHaveLength(1)
  })

  test("adds notification with different ID", () => {
    const a = makeNotif("notif-added-e1")
    const b = makeNotif("notif-updated-e2")
    const result = addNotification([a], b)
    expect(result).toHaveLength(2)
    expect(result[0].id).toBe("notif-updated-e2") // newest first
  })

  test("respects maxCount", () => {
    const list = [makeNotif("a"), makeNotif("b"), makeNotif("c")]
    const result = addNotification(list, makeNotif("d"), 3)
    expect(result).toHaveLength(3)
    expect(result[0].id).toBe("d")
    expect(result[2].id).toBe("b") // "c" dropped
  })
})
