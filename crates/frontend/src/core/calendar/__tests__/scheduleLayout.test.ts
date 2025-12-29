import { describe, expect, test } from "bun:test"
import {
  calculateDuration,
  calculateEntryWidth,
  calculateGridHeight,
  calculateScrollToHour,
  calculateTimePosition,
  detectOverlappingEntries,
  formatHourLabel,
  generateHourLabels,
  HOUR_HEIGHT_PX,
  HOURS_IN_DAY,
  parseTimeToMinutes,
  separateEntriesByType,
} from "../scheduleLayout"
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
    startTime: "10:00:00",
    endTime: "11:00:00",
    ...overrides,
  }
}

describe("parseTimeToMinutes", () => {
  test("parses HH:MM:SS format", () => {
    expect(parseTimeToMinutes("09:30:00")).toBe(9 * 60 + 30)
    expect(parseTimeToMinutes("14:45:00")).toBe(14 * 60 + 45)
    expect(parseTimeToMinutes("00:00:00")).toBe(0)
    expect(parseTimeToMinutes("23:59:59")).toBe(23 * 60 + 59)
  })

  test("parses HH:MM format", () => {
    expect(parseTimeToMinutes("09:30")).toBe(9 * 60 + 30)
    expect(parseTimeToMinutes("14:45")).toBe(14 * 60 + 45)
  })

  test("returns 0 for null or invalid input", () => {
    expect(parseTimeToMinutes(null)).toBe(0)
    expect(parseTimeToMinutes("")).toBe(0)
    expect(parseTimeToMinutes("invalid")).toBe(0)
  })
})

describe("calculateDuration", () => {
  test("calculates duration in minutes", () => {
    expect(calculateDuration("09:00:00", "10:00:00")).toBe(60)
    expect(calculateDuration("09:00:00", "10:30:00")).toBe(90)
    expect(calculateDuration("14:00:00", "14:15:00")).toBe(15)
  })

  test("handles overnight events", () => {
    // 11 PM to 1 AM = 2 hours
    expect(calculateDuration("23:00:00", "01:00:00")).toBe(2 * 60)
  })

  test("handles null inputs gracefully", () => {
    // When start is null (0), end is 10:00 = 600 minutes
    expect(calculateDuration(null, "10:00:00")).toBe(600)
    // When end is null (0), start is 10:00 = overnight = 1440 - 600 = 840 minutes
    expect(calculateDuration("10:00:00", null)).toBe(840)
    // Both null = 0 duration
    expect(calculateDuration(null, null)).toBe(0)
  })
})

describe("calculateTimePosition", () => {
  test("calculates position for 9 AM event", () => {
    const result = calculateTimePosition("09:00:00", "10:00:00")
    expect(result.top).toBe(9 * HOUR_HEIGHT_PX)
    expect(result.height).toBe(HOUR_HEIGHT_PX)
  })

  test("calculates position for 30-minute event", () => {
    const result = calculateTimePosition("09:00:00", "09:30:00")
    expect(result.top).toBe(9 * HOUR_HEIGHT_PX)
    expect(result.height).toBe(HOUR_HEIGHT_PX / 2)
  })

  test("enforces minimum height for very short events", () => {
    const result = calculateTimePosition("09:00:00", "09:05:00")
    expect(result.height).toBeGreaterThanOrEqual(HOUR_HEIGHT_PX / 4) // 15 min minimum
  })

  test("uses custom hour height", () => {
    const customHeight = 100
    const result = calculateTimePosition("09:00:00", "10:00:00", customHeight)
    expect(result.top).toBe(9 * customHeight)
    expect(result.height).toBe(customHeight)
  })
})

describe("calculateScrollToHour", () => {
  test("calculates scroll position for 8 AM", () => {
    expect(calculateScrollToHour(8)).toBe(8 * HOUR_HEIGHT_PX)
  })

  test("calculates scroll position for midnight", () => {
    expect(calculateScrollToHour(0)).toBe(0)
  })

  test("uses custom hour height", () => {
    expect(calculateScrollToHour(8, 100)).toBe(800)
  })
})

describe("separateEntriesByType", () => {
  test("separates entries by type", () => {
    const entries = [
      createEntry({ id: "1", isAllDay: true, isTimed: false }),
      createEntry({ id: "2", isMultiDay: true, isTimed: false }),
      createEntry({ id: "3", isTask: true, isTimed: false }),
      createEntry({ id: "4", isTimed: true }),
      createEntry({ id: "5", isTimed: true }),
    ]

    const result = separateEntriesByType(entries)

    expect(result.allDay).toHaveLength(1)
    expect(result.allDay[0].id).toBe("1")

    expect(result.multiDay).toHaveLength(1)
    expect(result.multiDay[0].id).toBe("2")

    expect(result.tasks).toHaveLength(1)
    expect(result.tasks[0].id).toBe("3")

    expect(result.timed).toHaveLength(2)
    expect(result.timed.map((e) => e.id)).toContain("4")
    expect(result.timed.map((e) => e.id)).toContain("5")
  })

  test("handles empty array", () => {
    const result = separateEntriesByType([])
    expect(result.allDay).toHaveLength(0)
    expect(result.multiDay).toHaveLength(0)
    expect(result.tasks).toHaveLength(0)
    expect(result.timed).toHaveLength(0)
  })
})

describe("detectOverlappingEntries", () => {
  test("assigns single column for non-overlapping entries", () => {
    const entries = [
      createEntry({ id: "1", startTime: "09:00:00", endTime: "10:00:00" }),
      createEntry({ id: "2", startTime: "11:00:00", endTime: "12:00:00" }),
    ]

    const result = detectOverlappingEntries(entries)

    expect(result.get("1")?.columnIndex).toBe(0)
    expect(result.get("1")?.totalColumns).toBe(1)
    expect(result.get("2")?.columnIndex).toBe(0)
    expect(result.get("2")?.totalColumns).toBe(1)
  })

  test("assigns multiple columns for overlapping entries", () => {
    const entries = [
      createEntry({ id: "1", startTime: "09:00:00", endTime: "11:00:00" }),
      createEntry({ id: "2", startTime: "10:00:00", endTime: "12:00:00" }),
    ]

    const result = detectOverlappingEntries(entries)

    expect(result.get("1")?.totalColumns).toBe(2)
    expect(result.get("2")?.totalColumns).toBe(2)
    // Different column indices
    expect(result.get("1")?.columnIndex).not.toBe(result.get("2")?.columnIndex)
  })

  test("handles three overlapping entries", () => {
    const entries = [
      createEntry({ id: "1", startTime: "09:00:00", endTime: "12:00:00" }),
      createEntry({ id: "2", startTime: "10:00:00", endTime: "11:00:00" }),
      createEntry({ id: "3", startTime: "10:30:00", endTime: "11:30:00" }),
    ]

    const result = detectOverlappingEntries(entries)

    // All should have 3 columns when overlapping
    const columns = new Set([
      result.get("1")?.columnIndex,
      result.get("2")?.columnIndex,
      result.get("3")?.columnIndex,
    ])
    // Should use different columns
    expect(columns.size).toBe(3)
  })

  test("handles empty array", () => {
    const result = detectOverlappingEntries([])
    expect(result.size).toBe(0)
  })
})

describe("calculateEntryWidth", () => {
  test("calculates full width for single column", () => {
    const result = calculateEntryWidth({ columnIndex: 0, totalColumns: 1 }, 300)
    expect(result.width).toBe(300)
    expect(result.left).toBe(0)
  })

  test("calculates half width for two columns", () => {
    const result1 = calculateEntryWidth({ columnIndex: 0, totalColumns: 2 }, 300)
    expect(result1.width).toBe(150)
    expect(result1.left).toBe(0)

    const result2 = calculateEntryWidth({ columnIndex: 1, totalColumns: 2 }, 300)
    expect(result2.width).toBe(150)
    expect(result2.left).toBe(150)
  })

  test("calculates third width for three columns", () => {
    const result = calculateEntryWidth({ columnIndex: 1, totalColumns: 3 }, 300)
    expect(result.width).toBe(100)
    expect(result.left).toBe(100)
  })
})

describe("calculateGridHeight", () => {
  test("calculates total grid height", () => {
    expect(calculateGridHeight()).toBe(HOURS_IN_DAY * HOUR_HEIGHT_PX)
  })

  test("uses custom hour height", () => {
    expect(calculateGridHeight(100)).toBe(HOURS_IN_DAY * 100)
  })
})

describe("formatHourLabel", () => {
  test("formats midnight", () => {
    expect(formatHourLabel(0)).toBe("12 AM")
  })

  test("formats morning hours", () => {
    expect(formatHourLabel(1)).toBe("1 AM")
    expect(formatHourLabel(9)).toBe("9 AM")
    expect(formatHourLabel(11)).toBe("11 AM")
  })

  test("formats noon", () => {
    expect(formatHourLabel(12)).toBe("12 PM")
  })

  test("formats afternoon/evening hours", () => {
    expect(formatHourLabel(13)).toBe("1 PM")
    expect(formatHourLabel(18)).toBe("6 PM")
    expect(formatHourLabel(23)).toBe("11 PM")
  })
})

describe("generateHourLabels", () => {
  test("generates 24 hour labels", () => {
    const labels = generateHourLabels()
    expect(labels).toHaveLength(24)
    expect(labels[0]).toBe("12 AM")
    expect(labels[12]).toBe("12 PM")
    expect(labels[23]).toBe("11 PM")
  })
})
