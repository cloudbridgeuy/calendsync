import { describe, expect, test } from "bun:test"
import { determineInitialView, selectCalendar } from "../selection"

describe("selectCalendar", () => {
  test("returns 'no_calendars' when availableCalendars is empty", () => {
    const result = selectCalendar("some-id", [])
    expect(result).toEqual({ type: "no_calendars" })
  })

  test("returns 'no_calendars' when storedCalendarId is null and no calendars", () => {
    const result = selectCalendar(null, [])
    expect(result).toEqual({ type: "no_calendars" })
  })

  test("returns 'use_stored' when storedCalendarId exists and is valid", () => {
    const calendars = [{ id: "cal-1" }, { id: "cal-2" }, { id: "cal-3" }]
    const result = selectCalendar("cal-2", calendars)
    expect(result).toEqual({ type: "use_stored", calendarId: "cal-2" })
  })

  test("returns 'use_stored' when storedCalendarId is the first calendar", () => {
    const calendars = [{ id: "cal-1" }, { id: "cal-2" }]
    const result = selectCalendar("cal-1", calendars)
    expect(result).toEqual({ type: "use_stored", calendarId: "cal-1" })
  })

  test("returns 'use_stored' when storedCalendarId is the last calendar", () => {
    const calendars = [{ id: "cal-1" }, { id: "cal-2" }, { id: "cal-3" }]
    const result = selectCalendar("cal-3", calendars)
    expect(result).toEqual({ type: "use_stored", calendarId: "cal-3" })
  })

  test("returns 'use_default' when storedCalendarId is null", () => {
    const calendars = [{ id: "cal-1" }, { id: "cal-2" }]
    const result = selectCalendar(null, calendars)
    expect(result).toEqual({ type: "use_default", calendarId: "cal-1" })
  })

  test("returns 'use_default' when storedCalendarId doesn't exist in available calendars", () => {
    const calendars = [{ id: "cal-1" }, { id: "cal-2" }]
    const result = selectCalendar("non-existent-id", calendars)
    expect(result).toEqual({ type: "use_default", calendarId: "cal-1" })
  })

  test("returns 'use_default' with first calendar when storedCalendarId is empty string", () => {
    const calendars = [{ id: "cal-1" }]
    // Empty string is falsy, so it will use default
    const result = selectCalendar("", calendars)
    expect(result).toEqual({ type: "use_default", calendarId: "cal-1" })
  })

  test("uses first calendar as default when only one calendar exists", () => {
    const calendars = [{ id: "only-cal" }]
    const result = selectCalendar(null, calendars)
    expect(result).toEqual({ type: "use_default", calendarId: "only-cal" })
  })
})

describe("determineInitialView", () => {
  test("returns 'loading_calendar' when sessionId exists and isValid is true", () => {
    const result = determineInitialView("session-123", true)
    expect(result).toBe("loading_calendar")
  })

  test("returns 'login' when sessionId is null", () => {
    const result = determineInitialView(null, true)
    expect(result).toBe("login")
  })

  test("returns 'login' when isValid is false", () => {
    const result = determineInitialView("session-123", false)
    expect(result).toBe("login")
  })

  test("returns 'login' when sessionId is null and isValid is false", () => {
    const result = determineInitialView(null, false)
    expect(result).toBe("login")
  })

  test("returns 'login' when sessionId is empty string", () => {
    // Empty string is falsy
    const result = determineInitialView("", true)
    expect(result).toBe("login")
  })

  // Exhaustive test of all combinations
  describe("all combinations", () => {
    const testCases: Array<{
      sessionId: string | null
      isValid: boolean
      expected: "login" | "loading_calendar"
    }> = [
      { sessionId: "valid-session", isValid: true, expected: "loading_calendar" },
      { sessionId: "valid-session", isValid: false, expected: "login" },
      { sessionId: null, isValid: true, expected: "login" },
      { sessionId: null, isValid: false, expected: "login" },
    ]

    for (const { sessionId, isValid, expected } of testCases) {
      const sessionDesc = sessionId ? `"${sessionId}"` : "null"
      test(`(${sessionDesc}, ${isValid}) => ${expected}`, () => {
        const result = determineInitialView(sessionId, isValid)
        expect(result).toBe(expected)
      })
    }
  })
})
