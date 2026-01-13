/**
 * Tests for form data conversion functions.
 */

import { describe, expect, test } from "bun:test"
import type { CreateEntryPayload } from "../../transport/types"
import { createPayloadToFormData, updatePayloadToFormData } from "../forms"
import type { ServerEntry } from "../types"

describe("createPayloadToFormData", () => {
  test("converts basic all-day entry", () => {
    const payload: CreateEntryPayload = {
      calendar_id: "cal123",
      title: "Team Meeting",
      date: "2024-01-15",
      all_day: true,
    }

    const params = createPayloadToFormData(payload)

    expect(params.get("calendar_id")).toBe("cal123")
    expect(params.get("title")).toBe("Team Meeting")
    expect(params.get("start_date")).toBe("2024-01-15")
    expect(params.get("entry_type")).toBe("all_day")
  })

  test("converts timed entry with start and end times", () => {
    const payload: CreateEntryPayload = {
      calendar_id: "cal123",
      title: "Standup",
      date: "2024-01-15",
      start_time: "09:00",
      end_time: "09:30",
    }

    const params = createPayloadToFormData(payload)

    expect(params.get("entry_type")).toBe("timed")
    expect(params.get("start_time")).toBe("09:00")
    expect(params.get("end_time")).toBe("09:30")
  })

  test("converts entry with explicit entry_type", () => {
    const payload: CreateEntryPayload = {
      calendar_id: "cal123",
      title: "Deploy Task",
      date: "2024-01-15",
      entry_type: "task",
    }

    const params = createPayloadToFormData(payload)

    expect(params.get("entry_type")).toBe("task")
  })

  test("includes optional description", () => {
    const payload: CreateEntryPayload = {
      calendar_id: "cal123",
      title: "Event",
      date: "2024-01-15",
      description: "Important details here",
    }

    const params = createPayloadToFormData(payload)

    expect(params.get("description")).toBe("Important details here")
  })

  test("defaults to all_day when no type indicators present", () => {
    const payload: CreateEntryPayload = {
      calendar_id: "cal123",
      title: "Event",
      date: "2024-01-15",
    }

    const params = createPayloadToFormData(payload)

    expect(params.get("entry_type")).toBe("all_day")
  })
})

describe("updatePayloadToFormData", () => {
  test("converts complete ServerEntry", () => {
    const payload: ServerEntry = {
      id: "entry123",
      calendarId: "cal123",
      kind: "entry",
      title: "Complete Entry",
      startDate: "2024-01-15",
      endDate: "2024-01-15",
      isAllDay: true,
      isTimed: false,
      isTask: false,
      isMultiDay: false,
      startTime: null,
      endTime: null,
      description: null,
      location: null,
      color: null,
      completed: false,
    }

    const params = updatePayloadToFormData(payload, "cal123")

    expect(params.get("calendar_id")).toBe("cal123")
    expect(params.get("title")).toBe("Complete Entry")
    expect(params.get("start_date")).toBe("2024-01-15")
    expect(params.get("entry_type")).toBe("all_day")
  })

  test("converts partial ServerEntry", () => {
    const payload: Partial<ServerEntry> = {
      title: "Partial Entry",
      startDate: "2024-01-15",
      isAllDay: false,
      startTime: "10:00",
      endTime: "11:00",
    }

    const params = updatePayloadToFormData(payload, "cal123")

    expect(params.get("calendar_id")).toBe("cal123")
    expect(params.get("title")).toBe("Partial Entry")
    expect(params.get("start_date")).toBe("2024-01-15")
  })

  test("derives entry type from flags for partial entry", () => {
    const payload: Partial<ServerEntry> = {
      title: "Timed Event",
      startDate: "2024-01-15",
      isTimed: true,
      startTime: "14:00",
    }

    const params = updatePayloadToFormData(payload, "cal123")

    expect(params.get("entry_type")).toBe("timed")
    expect(params.get("start_time")).toBe("14:00")
  })

  test("handles task entry with completed field", () => {
    const payload: Partial<ServerEntry> = {
      title: "Task Entry",
      startDate: "2024-01-15",
      isTask: true,
      completed: true,
    }

    const params = updatePayloadToFormData(payload, "cal123")

    expect(params.get("completed")).toBe("true")
  })
})
