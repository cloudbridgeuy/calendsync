import { describe, expect, test } from "bun:test"
import type { EntryFormData } from "../../calendar/modal"
import { deriveEntryType, formDataToEntry } from "../transformations"
import type { LocalEntry } from "../types"

function createTestFormData(overrides: Partial<EntryFormData> = {}): EntryFormData {
  return {
    title: "Test Entry",
    startDate: "2024-01-15",
    isAllDay: true,
    entryType: "all_day",
    ...overrides,
  }
}

function createTestLocalEntry(overrides: Partial<LocalEntry> = {}): LocalEntry {
  return {
    id: "test-id",
    calendarId: "cal-id",
    kind: "all_day",
    completed: false,
    isMultiDay: false,
    isAllDay: true,
    isTimed: false,
    isTask: false,
    title: "Existing Entry",
    description: null,
    location: null,
    color: "#ff0000",
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

describe("deriveEntryType", () => {
  test("returns all-day flags for all_day entry type", () => {
    const data = createTestFormData({ entryType: "all_day" })
    const result = deriveEntryType(data)

    expect(result).toEqual({
      isAllDay: true,
      isTimed: false,
      isTask: false,
      isMultiDay: false,
    })
  })

  test("returns timed flags for timed entry type", () => {
    const data = createTestFormData({ entryType: "timed" })
    const result = deriveEntryType(data)

    expect(result).toEqual({
      isAllDay: false,
      isTimed: true,
      isTask: false,
      isMultiDay: false,
    })
  })

  test("returns task flags for task entry type", () => {
    const data = createTestFormData({ entryType: "task" })
    const result = deriveEntryType(data)

    expect(result).toEqual({
      isAllDay: false,
      isTimed: false,
      isTask: true,
      isMultiDay: false,
    })
  })

  test("returns multi-day flags for multi_day entry type", () => {
    const data = createTestFormData({ entryType: "multi_day" })
    const result = deriveEntryType(data)

    expect(result).toEqual({
      isAllDay: false,
      isTimed: false,
      isTask: false,
      isMultiDay: true,
    })
  })
})

describe("formDataToEntry", () => {
  test("converts basic all-day entry", () => {
    const data = createTestFormData({
      title: "My Event",
      startDate: "2024-01-20",
      entryType: "all_day",
    })

    const result = formDataToEntry(data, "calendar-123")

    expect(result).toEqual({
      calendarId: "calendar-123",
      kind: "all_day",
      completed: false,
      isMultiDay: false,
      isAllDay: true,
      isTimed: false,
      isTask: false,
      title: "My Event",
      description: null,
      location: null,
      color: null,
      startDate: "2024-01-20",
      endDate: "2024-01-20",
      startTime: null,
      endTime: null,
    })
  })

  test("converts timed entry with start and end times", () => {
    const data = createTestFormData({
      title: "Meeting",
      startDate: "2024-01-20",
      entryType: "timed",
      startTime: "09:00",
      endTime: "10:30",
      isAllDay: false,
    })

    const result = formDataToEntry(data, "calendar-123")

    expect(result.isTimed).toBe(true)
    expect(result.isAllDay).toBe(false)
    expect(result.startTime).toBe("09:00")
    expect(result.endTime).toBe("10:30")
  })

  test("converts task entry with completed status", () => {
    const data = createTestFormData({
      title: "My Task",
      startDate: "2024-01-20",
      entryType: "task",
      completed: true,
    })

    const result = formDataToEntry(data, "calendar-123")

    expect(result.isTask).toBe(true)
    expect(result.completed).toBe(true)
  })

  test("converts multi-day entry with end date", () => {
    const data = createTestFormData({
      title: "Vacation",
      startDate: "2024-01-20",
      endDate: "2024-01-25",
      entryType: "multi_day",
    })

    const result = formDataToEntry(data, "calendar-123")

    expect(result.isMultiDay).toBe(true)
    expect(result.startDate).toBe("2024-01-20")
    expect(result.endDate).toBe("2024-01-25")
  })

  test("uses startDate as endDate when endDate not provided", () => {
    const data = createTestFormData({
      startDate: "2024-01-20",
      endDate: undefined,
    })

    const result = formDataToEntry(data, "calendar-123")

    expect(result.endDate).toBe("2024-01-20")
  })

  test("includes optional description and location", () => {
    const data = createTestFormData({
      description: "Important meeting about Q1 goals",
      location: "Conference Room A",
    })

    const result = formDataToEntry(data, "calendar-123")

    expect(result.description).toBe("Important meeting about Q1 goals")
    expect(result.location).toBe("Conference Room A")
  })

  test("sets null for undefined optional fields", () => {
    const data = createTestFormData({
      description: undefined,
      location: undefined,
    })

    const result = formDataToEntry(data, "calendar-123")

    expect(result.description).toBeNull()
    expect(result.location).toBeNull()
  })

  test("preserves color from existing entry", () => {
    const data = createTestFormData()
    const existingEntry = createTestLocalEntry({ color: "#00ff00" })

    const result = formDataToEntry(data, "calendar-123", existingEntry)

    expect(result.color).toBe("#00ff00")
  })

  test("sets null color when no existing entry", () => {
    const data = createTestFormData()

    const result = formDataToEntry(data, "calendar-123")

    expect(result.color).toBeNull()
  })

  test("defaults completed to false when undefined", () => {
    const data = createTestFormData({
      completed: undefined,
    })

    const result = formDataToEntry(data, "calendar-123")

    expect(result.completed).toBe(false)
  })

  test("sets null for undefined time fields", () => {
    const data = createTestFormData({
      startTime: undefined,
      endTime: undefined,
    })

    const result = formDataToEntry(data, "calendar-123")

    expect(result.startTime).toBeNull()
    expect(result.endTime).toBeNull()
  })
})
