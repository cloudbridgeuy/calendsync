import { describe, expect, test } from "bun:test"
import type { EntryFormData } from "../modal"
import {
  buildCalendarUrl,
  buildModalUrl,
  createDefaultFormData,
  entryToFormData,
  FOCUSABLE_SELECTOR,
  formDataToApiPayload,
  getNextFocusIndex,
  parseModalUrl,
  validateFormData,
} from "../modal"
import type { ServerEntry } from "../types"

// Helper to create a minimal ServerEntry for testing
function createServerEntry(overrides: Partial<ServerEntry> = {}): ServerEntry {
  return {
    id: "test-entry-id",
    calendarId: "test-calendar-id",
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
    date: "2024-01-15",
    startTime: null,
    endTime: null,
    multiDayStart: null,
    multiDayEnd: null,
    multiDayStartDate: null,
    multiDayEndDate: null,
    ...overrides,
  }
}

describe("parseModalUrl", () => {
  test("returns create mode for /entry path", () => {
    const result = parseModalUrl("/calendar/abc-123/entry", "")
    expect(result).toEqual({ mode: "create" })
  })

  test("returns edit mode with entryId from query param", () => {
    const result = parseModalUrl("/calendar/abc-123/entry", "?entry_id=xyz-789")
    expect(result).toEqual({ mode: "edit", entryId: "xyz-789" })
  })

  test("handles entry_id with special characters", () => {
    const result = parseModalUrl("/calendar/abc-123/entry", "?entry_id=abc%20def")
    expect(result).toEqual({ mode: "edit", entryId: "abc def" })
  })

  test("returns null for calendar URL without /entry", () => {
    const result = parseModalUrl("/calendar/abc-123", "")
    expect(result).toBeNull()
  })

  test("returns null for non-calendar URLs", () => {
    expect(parseModalUrl("/", "")).toBeNull()
    expect(parseModalUrl("/settings", "")).toBeNull()
    expect(parseModalUrl("/calendar", "")).toBeNull()
  })

  test("returns null for paths with extra segments after /entry", () => {
    const result = parseModalUrl("/calendar/abc-123/entry/extra", "")
    expect(result).toBeNull()
  })

  test("ignores other query parameters", () => {
    const result = parseModalUrl("/calendar/abc-123/entry", "?other=value")
    expect(result).toEqual({ mode: "create" })
  })
})

describe("buildModalUrl", () => {
  test("builds create mode URL", () => {
    const result = buildModalUrl("abc-123", "create")
    expect(result).toBe("/calendar/abc-123/entry")
  })

  test("builds edit mode URL with entry_id", () => {
    const result = buildModalUrl("abc-123", "edit", "xyz-789")
    expect(result).toBe("/calendar/abc-123/entry?entry_id=xyz-789")
  })

  test("encodes special characters in entry_id", () => {
    const result = buildModalUrl("abc-123", "edit", "id with spaces")
    expect(result).toBe("/calendar/abc-123/entry?entry_id=id%20with%20spaces")
  })

  test("ignores entryId for create mode", () => {
    const result = buildModalUrl("abc-123", "create", "xyz-789")
    expect(result).toBe("/calendar/abc-123/entry")
  })
})

describe("buildCalendarUrl", () => {
  test("builds calendar URL", () => {
    const result = buildCalendarUrl("abc-123")
    expect(result).toBe("/calendar/abc-123")
  })
})

describe("entryToFormData", () => {
  test("converts all-day entry", () => {
    const entry = createServerEntry({
      isAllDay: true,
      isTimed: false,
      isTask: false,
      isMultiDay: false,
    })
    const result = entryToFormData(entry)

    expect(result.title).toBe("Test Entry")
    expect(result.date).toBe("2024-01-15")
    expect(result.isAllDay).toBe(true)
    expect(result.entryType).toBe("all_day")
    expect(result.startTime).toBeUndefined()
    expect(result.endTime).toBeUndefined()
  })

  test("converts timed entry", () => {
    const entry = createServerEntry({
      isAllDay: false,
      isTimed: true,
      isTask: false,
      isMultiDay: false,
      startTime: "09:00",
      endTime: "10:30",
    })
    const result = entryToFormData(entry)

    expect(result.entryType).toBe("timed")
    expect(result.isAllDay).toBe(false)
    expect(result.startTime).toBe("09:00")
    expect(result.endTime).toBe("10:30")
  })

  test("converts task entry with completed status", () => {
    const entry = createServerEntry({
      isAllDay: false,
      isTimed: false,
      isTask: true,
      isMultiDay: false,
      completed: true,
    })
    const result = entryToFormData(entry)

    expect(result.entryType).toBe("task")
    expect(result.completed).toBe(true)
  })

  test("converts incomplete task entry", () => {
    const entry = createServerEntry({
      isAllDay: false,
      isTimed: false,
      isTask: true,
      isMultiDay: false,
      completed: false,
    })
    const result = entryToFormData(entry)

    expect(result.entryType).toBe("task")
    expect(result.completed).toBe(false)
  })

  test("does not include completed for non-task entries", () => {
    const entry = createServerEntry({
      isAllDay: true,
      isTimed: false,
      isTask: false,
      isMultiDay: false,
      completed: false,
    })
    const result = entryToFormData(entry)

    expect(result.completed).toBeUndefined()
  })

  test("converts multi-day entry", () => {
    const entry = createServerEntry({
      isAllDay: false,
      isTimed: false,
      isTask: false,
      isMultiDay: true,
      multiDayStartDate: "2024-01-15",
      multiDayEndDate: "2024-01-18",
    })
    const result = entryToFormData(entry)

    expect(result.entryType).toBe("multi_day")
    expect(result.endDate).toBe("2024-01-18")
  })

  test("preserves optional fields", () => {
    const entry = createServerEntry({
      description: "Test description",
      location: "Test location",
    })
    const result = entryToFormData(entry)

    expect(result.description).toBe("Test description")
    expect(result.location).toBe("Test location")
  })

  test("converts null fields to undefined", () => {
    const entry = createServerEntry({
      description: null,
      location: null,
      startTime: null,
      endTime: null,
    })
    const result = entryToFormData(entry)

    expect(result.description).toBeUndefined()
    expect(result.location).toBeUndefined()
    expect(result.startTime).toBeUndefined()
    expect(result.endTime).toBeUndefined()
  })
})

describe("createDefaultFormData", () => {
  test("creates empty form data without default date", () => {
    const result = createDefaultFormData()

    expect(result.title).toBe("")
    expect(result.date).toBe("")
    expect(result.isAllDay).toBe(true)
    expect(result.entryType).toBe("all_day")
  })

  test("creates form data with default date", () => {
    const result = createDefaultFormData("2024-01-15")

    expect(result.date).toBe("2024-01-15")
    expect(result.isAllDay).toBe(true)
  })
})

describe("formDataToApiPayload", () => {
  test("converts all-day entry", () => {
    const data: EntryFormData = {
      title: "Test Event",
      date: "2024-01-15",
      isAllDay: true,
      entryType: "all_day",
    }
    const result = formDataToApiPayload(data, "cal-123")

    expect(result.get("calendar_id")).toBe("cal-123")
    expect(result.get("title")).toBe("Test Event")
    expect(result.get("date")).toBe("2024-01-15")
    expect(result.get("entry_type")).toBe("all_day")
    expect(result.get("start_time")).toBeNull()
    expect(result.get("end_time")).toBeNull()
  })

  test("converts timed entry with times", () => {
    const data: EntryFormData = {
      title: "Meeting",
      date: "2024-01-15",
      isAllDay: false,
      entryType: "timed",
      startTime: "09:00",
      endTime: "10:30",
    }
    const result = formDataToApiPayload(data, "cal-123")

    expect(result.get("entry_type")).toBe("timed")
    expect(result.get("start_time")).toBe("09:00")
    expect(result.get("end_time")).toBe("10:30")
  })

  test("includes optional description and location", () => {
    const data: EntryFormData = {
      title: "Event",
      date: "2024-01-15",
      isAllDay: true,
      entryType: "all_day",
      description: "A description",
      location: "A location",
    }
    const result = formDataToApiPayload(data, "cal-123")

    expect(result.get("description")).toBe("A description")
    expect(result.get("location")).toBe("A location")
  })

  test("excludes empty optional fields", () => {
    const data: EntryFormData = {
      title: "Event",
      date: "2024-01-15",
      isAllDay: true,
      entryType: "all_day",
      description: "",
      location: "",
    }
    const result = formDataToApiPayload(data, "cal-123")

    expect(result.get("description")).toBeNull()
    expect(result.get("location")).toBeNull()
  })

  test("includes end_date for multi-day entries", () => {
    const data: EntryFormData = {
      title: "Conference",
      date: "2024-01-15",
      isAllDay: false,
      entryType: "multi_day",
      endDate: "2024-01-18",
    }
    const result = formDataToApiPayload(data, "cal-123")

    expect(result.get("entry_type")).toBe("multi_day")
    expect(result.get("end_date")).toBe("2024-01-18")
  })

  test("ignores end_date for non-multi-day entries", () => {
    const data: EntryFormData = {
      title: "Event",
      date: "2024-01-15",
      isAllDay: true,
      entryType: "all_day",
      endDate: "2024-01-18", // Should be ignored
    }
    const result = formDataToApiPayload(data, "cal-123")

    expect(result.get("end_date")).toBeNull()
  })

  test("includes completed for task entries", () => {
    const data: EntryFormData = {
      title: "Task",
      date: "2024-01-15",
      isAllDay: false,
      entryType: "task",
      completed: true,
    }
    const result = formDataToApiPayload(data, "cal-123")

    expect(result.get("entry_type")).toBe("task")
    expect(result.get("completed")).toBe("true")
  })

  test("includes completed=false for incomplete tasks", () => {
    const data: EntryFormData = {
      title: "Task",
      date: "2024-01-15",
      isAllDay: false,
      entryType: "task",
      completed: false,
    }
    const result = formDataToApiPayload(data, "cal-123")

    expect(result.get("completed")).toBe("false")
  })

  test("ignores completed for non-task entries", () => {
    const data: EntryFormData = {
      title: "Event",
      date: "2024-01-15",
      isAllDay: true,
      entryType: "all_day",
      completed: true, // Should be ignored
    }
    const result = formDataToApiPayload(data, "cal-123")

    expect(result.get("completed")).toBeNull()
  })
})

describe("validateFormData", () => {
  test("validates valid all-day entry", () => {
    const data: EntryFormData = {
      title: "Valid Event",
      date: "2024-01-15",
      isAllDay: true,
      entryType: "all_day",
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(true)
    expect(result.errors).toHaveLength(0)
  })

  test("validates valid timed entry", () => {
    const data: EntryFormData = {
      title: "Meeting",
      date: "2024-01-15",
      isAllDay: false,
      entryType: "timed",
      startTime: "09:00",
      endTime: "10:30",
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(true)
  })

  test("requires title", () => {
    const data: EntryFormData = {
      title: "",
      date: "2024-01-15",
      isAllDay: true,
      entryType: "all_day",
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(false)
    expect(result.errors).toContain("Title is required")
  })

  test("requires title with content (not just whitespace)", () => {
    const data: EntryFormData = {
      title: "   ",
      date: "2024-01-15",
      isAllDay: true,
      entryType: "all_day",
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(false)
    expect(result.errors).toContain("Title is required")
  })

  test("requires date", () => {
    const data: EntryFormData = {
      title: "Event",
      date: "",
      isAllDay: true,
      entryType: "all_day",
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(false)
    expect(result.errors).toContain("Date is required")
  })

  test("validates date format", () => {
    const data: EntryFormData = {
      title: "Event",
      date: "01-15-2024", // Wrong format
      isAllDay: true,
      entryType: "all_day",
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(false)
    expect(result.errors).toContain("Date must be in YYYY-MM-DD format")
  })

  test("validates start time format", () => {
    const data: EntryFormData = {
      title: "Event",
      date: "2024-01-15",
      isAllDay: false,
      entryType: "timed",
      startTime: "9:00", // Missing leading zero
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(false)
    expect(result.errors).toContain("Start time must be in HH:MM format")
  })

  test("validates end time format", () => {
    const data: EntryFormData = {
      title: "Event",
      date: "2024-01-15",
      isAllDay: false,
      entryType: "timed",
      startTime: "09:00",
      endTime: "10:3", // Invalid format
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(false)
    expect(result.errors).toContain("End time must be in HH:MM format")
  })

  test("validates end time is after start time", () => {
    const data: EntryFormData = {
      title: "Event",
      date: "2024-01-15",
      isAllDay: false,
      entryType: "timed",
      startTime: "10:00",
      endTime: "09:00", // Before start
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(false)
    expect(result.errors).toContain("End time must be after start time")
  })

  test("rejects same start and end time", () => {
    const data: EntryFormData = {
      title: "Event",
      date: "2024-01-15",
      isAllDay: false,
      entryType: "timed",
      startTime: "10:00",
      endTime: "10:00", // Same as start
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(false)
    expect(result.errors).toContain("End time must be after start time")
  })

  test("requires end date for multi-day entries", () => {
    const data: EntryFormData = {
      title: "Conference",
      date: "2024-01-15",
      isAllDay: false,
      entryType: "multi_day",
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(false)
    expect(result.errors).toContain("End date is required for multi-day entries")
  })

  test("validates end date is after start date for multi-day", () => {
    const data: EntryFormData = {
      title: "Conference",
      date: "2024-01-15",
      isAllDay: false,
      entryType: "multi_day",
      endDate: "2024-01-14", // Before start
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(false)
    expect(result.errors).toContain("End date must be after start date")
  })

  test("rejects same start and end date for multi-day", () => {
    const data: EntryFormData = {
      title: "Conference",
      date: "2024-01-15",
      isAllDay: false,
      entryType: "multi_day",
      endDate: "2024-01-15", // Same as start
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(false)
    expect(result.errors).toContain("End date must be after start date")
  })

  test("allows end date only for multi-day entries", () => {
    const data: EntryFormData = {
      title: "Event",
      date: "2024-01-15",
      isAllDay: true,
      entryType: "all_day",
      endDate: "2024-01-18", // Should be ignored, not cause error
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(true)
  })

  test("collects multiple errors", () => {
    const data: EntryFormData = {
      title: "",
      date: "",
      isAllDay: false,
      entryType: "multi_day",
    }
    const result = validateFormData(data)

    expect(result.valid).toBe(false)
    expect(result.errors.length).toBeGreaterThan(1)
    expect(result.errors).toContain("Title is required")
    expect(result.errors).toContain("Date is required")
  })
})

describe("FOCUSABLE_SELECTOR", () => {
  test("is a valid CSS selector string", () => {
    expect(typeof FOCUSABLE_SELECTOR).toBe("string")
    expect(FOCUSABLE_SELECTOR.length).toBeGreaterThan(0)
  })

  test("includes common focusable elements", () => {
    expect(FOCUSABLE_SELECTOR).toContain("button")
    expect(FOCUSABLE_SELECTOR).toContain("input")
    expect(FOCUSABLE_SELECTOR).toContain("select")
    expect(FOCUSABLE_SELECTOR).toContain("textarea")
  })

  test("excludes disabled elements", () => {
    expect(FOCUSABLE_SELECTOR).toContain(":not([disabled])")
  })
})

describe("getNextFocusIndex", () => {
  test("returns next index for forward direction", () => {
    expect(getNextFocusIndex(0, 5, "forward")).toBe(1)
    expect(getNextFocusIndex(2, 5, "forward")).toBe(3)
  })

  test("wraps to start when at end (forward)", () => {
    expect(getNextFocusIndex(4, 5, "forward")).toBe(0)
  })

  test("returns previous index for backward direction", () => {
    expect(getNextFocusIndex(2, 5, "backward")).toBe(1)
    expect(getNextFocusIndex(4, 5, "backward")).toBe(3)
  })

  test("wraps to end when at start (backward)", () => {
    expect(getNextFocusIndex(0, 5, "backward")).toBe(4)
  })

  test("returns -1 for empty list", () => {
    expect(getNextFocusIndex(0, 0, "forward")).toBe(-1)
    expect(getNextFocusIndex(0, 0, "backward")).toBe(-1)
  })

  test("handles single element", () => {
    expect(getNextFocusIndex(0, 1, "forward")).toBe(0)
    expect(getNextFocusIndex(0, 1, "backward")).toBe(0)
  })
})
