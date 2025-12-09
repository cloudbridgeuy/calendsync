import { describe, expect, test } from "bun:test"
import { getDayDisplayInfo, isDayToday } from "../dayContainer"

describe("isDayToday", () => {
  test("returns true for today's date", () => {
    const today = new Date()
    expect(isDayToday(today)).toBe(true)
  })

  test("returns false for yesterday", () => {
    const yesterday = new Date()
    yesterday.setDate(yesterday.getDate() - 1)
    expect(isDayToday(yesterday)).toBe(false)
  })

  test("returns false for tomorrow", () => {
    const tomorrow = new Date()
    tomorrow.setDate(tomorrow.getDate() + 1)
    expect(isDayToday(tomorrow)).toBe(false)
  })

  test("returns false for a date in different year", () => {
    const differentYear = new Date()
    differentYear.setFullYear(differentYear.getFullYear() - 1)
    expect(isDayToday(differentYear)).toBe(false)
  })

  test("returns true for today with different time", () => {
    const todayMorning = new Date()
    todayMorning.setHours(8, 0, 0, 0)
    expect(isDayToday(todayMorning)).toBe(true)
  })
})

describe("getDayDisplayInfo", () => {
  test("returns correct dayNumber (1-31)", () => {
    const date = new Date("2025-01-15T12:00:00")
    const info = getDayDisplayInfo(date)
    expect(info.dayNumber).toBe(15)
  })

  test("returns correct dayName (uppercase)", () => {
    // January 15, 2025 is a Wednesday
    const wednesday = new Date("2025-01-15T12:00:00")
    const info = getDayDisplayInfo(wednesday)
    expect(info.dayName).toBe("WED")
  })

  test("returns correct dayName for each day of week", () => {
    // Week starting January 5, 2025 (Sunday)
    const testCases = [
      { date: "2025-01-05", expected: "SUN" }, // Sunday
      { date: "2025-01-06", expected: "MON" }, // Monday
      { date: "2025-01-07", expected: "TUE" }, // Tuesday
      { date: "2025-01-08", expected: "WED" }, // Wednesday
      { date: "2025-01-09", expected: "THU" }, // Thursday
      { date: "2025-01-10", expected: "FRI" }, // Friday
      { date: "2025-01-11", expected: "SAT" }, // Saturday
    ]

    testCases.forEach(({ date, expected }) => {
      const info = getDayDisplayInfo(new Date(date))
      expect(info.dayName).toBe(expected)
    })
  })

  test("returns isToday=true for today", () => {
    const today = new Date()
    const info = getDayDisplayInfo(today)
    expect(info.isToday).toBe(true)
  })

  test("returns isToday=false for other dates", () => {
    const yesterday = new Date()
    yesterday.setDate(yesterday.getDate() - 1)
    const info = getDayDisplayInfo(yesterday)
    expect(info.isToday).toBe(false)
  })

  test("works for dates at year boundary (Dec 31)", () => {
    const lastDayOfYear = new Date("2024-12-31T23:59:59")
    const info = getDayDisplayInfo(lastDayOfYear)
    expect(info.dayNumber).toBe(31)
    expect(info.dayName).toBe("TUE") // Dec 31, 2024 is Tuesday
    expect(info.isToday).toBe(false)
  })

  test("works for dates at year boundary (Jan 1)", () => {
    const firstDayOfYear = new Date("2025-01-01T00:00:00")
    const info = getDayDisplayInfo(firstDayOfYear)
    expect(info.dayNumber).toBe(1)
    expect(info.dayName).toBe("WED") // Jan 1, 2025 is Wednesday
    expect(info.isToday).toBe(false)
  })

  test("handles leap year date (Feb 29)", () => {
    const leapDay = new Date("2024-02-29T12:00:00")
    const info = getDayDisplayInfo(leapDay)
    expect(info.dayNumber).toBe(29)
    expect(info.dayName).toBe("THU") // Feb 29, 2024 is Thursday
    expect(info.isToday).toBe(false)
  })

  test("handles start of month", () => {
    const firstOfMonth = new Date("2025-06-01T00:00:00")
    const info = getDayDisplayInfo(firstOfMonth)
    expect(info.dayNumber).toBe(1)
    expect(info.dayName).toBe("SUN") // June 1, 2025 is Sunday
    expect(info.isToday).toBe(false)
  })

  test("handles end of month with 31 days", () => {
    const endOfMonth = new Date("2025-01-31T23:59:59")
    const info = getDayDisplayInfo(endOfMonth)
    expect(info.dayNumber).toBe(31)
    expect(info.dayName).toBe("FRI") // Jan 31, 2025 is Friday
    expect(info.isToday).toBe(false)
  })

  test("handles end of month with 30 days", () => {
    const endOfMonth = new Date("2025-04-30T23:59:59")
    const info = getDayDisplayInfo(endOfMonth)
    expect(info.dayNumber).toBe(30)
    expect(info.dayName).toBe("WED") // April 30, 2025 is Wednesday
    expect(info.isToday).toBe(false)
  })

  test("handles February 28 in non-leap year", () => {
    const endOfFeb = new Date("2025-02-28T23:59:59")
    const info = getDayDisplayInfo(endOfFeb)
    expect(info.dayNumber).toBe(28)
    expect(info.dayName).toBe("FRI") // Feb 28, 2025 is Friday
    expect(info.isToday).toBe(false)
  })

  test("returns DayDisplayInfo with all required fields", () => {
    const date = new Date("2025-01-15T12:00:00")
    const info = getDayDisplayInfo(date)

    // Check that all required fields are present
    expect(info).toHaveProperty("dayNumber")
    expect(info).toHaveProperty("dayName")
    expect(info).toHaveProperty("isToday")

    // Check types
    expect(typeof info.dayNumber).toBe("number")
    expect(typeof info.dayName).toBe("string")
    expect(typeof info.isToday).toBe("boolean")
  })

  test("dayName is always uppercase", () => {
    const dates = [
      new Date("2025-01-05"), // Sunday
      new Date("2025-01-06"), // Monday
      new Date("2025-01-07"), // Tuesday
    ]

    dates.forEach((date) => {
      const info = getDayDisplayInfo(date)
      expect(info.dayName).toBe(info.dayName.toUpperCase())
    })
  })

  test("handles dates with different times on same day", () => {
    const morning = new Date("2025-03-15T08:00:00")
    const afternoon = new Date("2025-03-15T14:30:00")
    const night = new Date("2025-03-15T23:45:00")

    const infoMorning = getDayDisplayInfo(morning)
    const infoAfternoon = getDayDisplayInfo(afternoon)
    const infoNight = getDayDisplayInfo(night)

    // All should have same day info regardless of time
    expect(infoMorning.dayNumber).toBe(15)
    expect(infoAfternoon.dayNumber).toBe(15)
    expect(infoNight.dayNumber).toBe(15)

    expect(infoMorning.dayName).toBe("SAT")
    expect(infoAfternoon.dayName).toBe("SAT")
    expect(infoNight.dayName).toBe("SAT")
  })
})
