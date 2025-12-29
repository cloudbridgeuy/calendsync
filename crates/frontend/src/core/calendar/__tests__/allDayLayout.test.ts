import { describe, expect, test } from "bun:test"
import {
  categorizeAllDayEntries,
  computeAllDaySummary,
  formatOverflowToggle,
  formatTasksToggle,
  MAX_VISIBLE_ALL_DAY,
} from "../allDayLayout"
import type { ServerEntry } from "../types"

// Helper to create test entries
function createEntry(overrides: Partial<ServerEntry> = {}): ServerEntry {
  return {
    id: "test-id",
    calendarId: "cal-1",
    kind: "event",
    completed: false,
    isMultiDay: false,
    isAllDay: false,
    isTimed: true,
    isTask: false,
    title: "Test Entry",
    description: null,
    location: null,
    color: null,
    startDate: "2024-01-15",
    endDate: "2024-01-15",
    startTime: "10:00",
    endTime: "11:00",
    ...overrides,
  }
}

describe("categorizeAllDayEntries", () => {
  test("separates tasks from events", () => {
    const entries = [
      createEntry({ id: "1", isAllDay: true, isTask: false }),
      createEntry({ id: "2", isTask: true, isAllDay: false }),
      createEntry({ id: "3", isMultiDay: true, isTask: false }),
    ]

    const result = categorizeAllDayEntries(entries)

    expect(result.events.length).toBe(2)
    expect(result.tasks.length).toBe(1)
    expect(result.tasks[0].id).toBe("2")
  })

  test("sorts multi-day events before all-day events", () => {
    const entries = [
      createEntry({ id: "1", isAllDay: true, isMultiDay: false }),
      createEntry({ id: "2", isAllDay: false, isMultiDay: true }),
      createEntry({ id: "3", isAllDay: true, isMultiDay: false }),
    ]

    const result = categorizeAllDayEntries(entries)

    expect(result.events[0].id).toBe("2") // multi-day first
    expect(result.events[1].isAllDay).toBe(true)
    expect(result.events[2].isAllDay).toBe(true)
  })

  test("returns empty arrays for no entries", () => {
    const result = categorizeAllDayEntries([])

    expect(result.events.length).toBe(0)
    expect(result.tasks.length).toBe(0)
  })

  test("ignores timed entries", () => {
    const entries = [
      createEntry({ id: "1", isTimed: true, isAllDay: false, isMultiDay: false, isTask: false }),
    ]

    const result = categorizeAllDayEntries(entries)

    expect(result.events.length).toBe(0)
    expect(result.tasks.length).toBe(0)
  })
})

describe("computeAllDaySummary", () => {
  test("shows all events when count <= MAX_VISIBLE", () => {
    const entries = [
      createEntry({ id: "1", isAllDay: true }),
      createEntry({ id: "2", isAllDay: true }),
    ]

    const result = computeAllDaySummary(entries, false)

    expect(result.visibleEvents.length).toBe(2)
    expect(result.hiddenEventCount).toBe(0)
  })

  test("limits visible events when collapsed", () => {
    const entries = [
      createEntry({ id: "1", isAllDay: true }),
      createEntry({ id: "2", isAllDay: true }),
      createEntry({ id: "3", isAllDay: true }),
      createEntry({ id: "4", isAllDay: true }),
      createEntry({ id: "5", isAllDay: true }),
    ]

    const result = computeAllDaySummary(entries, false)

    expect(result.visibleEvents.length).toBe(MAX_VISIBLE_ALL_DAY)
    expect(result.hiddenEventCount).toBe(2)
  })

  test("shows all events when expanded", () => {
    const entries = [
      createEntry({ id: "1", isAllDay: true }),
      createEntry({ id: "2", isAllDay: true }),
      createEntry({ id: "3", isAllDay: true }),
      createEntry({ id: "4", isAllDay: true }),
      createEntry({ id: "5", isAllDay: true }),
    ]

    const result = computeAllDaySummary(entries, true)

    expect(result.visibleEvents.length).toBe(5)
    expect(result.hiddenEventCount).toBe(0)
  })

  test("includes tasks in summary regardless of overflow state", () => {
    const entries = [
      createEntry({ id: "1", isAllDay: true }),
      createEntry({ id: "2", isTask: true }),
      createEntry({ id: "3", isTask: true }),
    ]

    const result = computeAllDaySummary(entries, false)

    expect(result.tasks.length).toBe(2)
  })
})

describe("formatOverflowToggle", () => {
  test("returns null when no hidden entries and not expanded", () => {
    expect(formatOverflowToggle(0, false)).toBeNull()
  })

  test("returns formatted string when hidden entries exist", () => {
    expect(formatOverflowToggle(5, false)).toBe("(+5 more)")
    expect(formatOverflowToggle(1, false)).toBe("(+1 more)")
  })

  test("returns 'Show less' when expanded", () => {
    expect(formatOverflowToggle(0, true)).toBe("Show less")
    expect(formatOverflowToggle(5, true)).toBe("Show less")
  })
})

describe("formatTasksToggle", () => {
  test("returns null when no tasks", () => {
    expect(formatTasksToggle(0, false)).toBeNull()
    expect(formatTasksToggle(0, true)).toBeNull()
  })

  test("returns singular form for 1 task", () => {
    expect(formatTasksToggle(1, false)).toBe("(1 task)")
    expect(formatTasksToggle(1, true)).toBe("1 task")
  })

  test("returns plural form for multiple tasks", () => {
    expect(formatTasksToggle(2, false)).toBe("(2 tasks)")
    expect(formatTasksToggle(5, true)).toBe("5 tasks")
  })

  test("includes parentheses when collapsed, excludes when expanded", () => {
    expect(formatTasksToggle(3, false)).toBe("(3 tasks)")
    expect(formatTasksToggle(3, true)).toBe("3 tasks")
  })
})

describe("MAX_VISIBLE_ALL_DAY", () => {
  test("is set to 3", () => {
    expect(MAX_VISIBLE_ALL_DAY).toBe(3)
  })
})
