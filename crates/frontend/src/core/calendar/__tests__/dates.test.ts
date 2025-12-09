import { describe, expect, test } from "bun:test"
import {
  addDays,
  formatDateKey,
  getDateRange,
  getDatesAround,
  getDayOfMonth,
  getDayOfWeek,
  getMonth,
  getYear,
  isSameDay,
  isToday,
  parseDateKey,
  startOfDay,
} from "../dates"

describe("addDays", () => {
  test("adds positive days", () => {
    const date = new Date(2024, 0, 15) // Jan 15, 2024
    const result = addDays(date, 5)
    expect(result.getDate()).toBe(20)
    expect(result.getMonth()).toBe(0)
  })

  test("subtracts days with negative value", () => {
    const date = new Date(2024, 0, 15)
    const result = addDays(date, -5)
    expect(result.getDate()).toBe(10)
  })

  test("handles month boundaries", () => {
    const date = new Date(2024, 0, 31) // Jan 31
    const result = addDays(date, 1)
    expect(result.getDate()).toBe(1)
    expect(result.getMonth()).toBe(1) // February
  })

  test("does not mutate original date", () => {
    const date = new Date(2024, 0, 15)
    const originalTime = date.getTime()
    addDays(date, 5)
    expect(date.getTime()).toBe(originalTime)
  })
})

describe("isSameDay", () => {
  test("returns true for same day", () => {
    const d1 = new Date(2024, 0, 15, 10, 30)
    const d2 = new Date(2024, 0, 15, 14, 45)
    expect(isSameDay(d1, d2)).toBe(true)
  })

  test("returns false for different days", () => {
    const d1 = new Date(2024, 0, 15)
    const d2 = new Date(2024, 0, 16)
    expect(isSameDay(d1, d2)).toBe(false)
  })

  test("returns false for different months", () => {
    const d1 = new Date(2024, 0, 15)
    const d2 = new Date(2024, 1, 15)
    expect(isSameDay(d1, d2)).toBe(false)
  })

  test("returns false for different years", () => {
    const d1 = new Date(2024, 0, 15)
    const d2 = new Date(2025, 0, 15)
    expect(isSameDay(d1, d2)).toBe(false)
  })
})

describe("formatDateKey", () => {
  test("formats date as YYYY-MM-DD", () => {
    const date = new Date(2024, 0, 15)
    expect(formatDateKey(date)).toBe("2024-01-15")
  })

  test("pads single digit months", () => {
    const date = new Date(2024, 5, 15)
    expect(formatDateKey(date)).toBe("2024-06-15")
  })

  test("pads single digit days", () => {
    const date = new Date(2024, 0, 5)
    expect(formatDateKey(date)).toBe("2024-01-05")
  })
})

describe("parseDateKey", () => {
  test("parses YYYY-MM-DD string", () => {
    const result = parseDateKey("2024-01-15")
    expect(result.getFullYear()).toBe(2024)
    expect(result.getMonth()).toBe(0)
    expect(result.getDate()).toBe(15)
  })

  test("sets time to midnight", () => {
    const result = parseDateKey("2024-01-15")
    expect(result.getHours()).toBe(0)
    expect(result.getMinutes()).toBe(0)
    expect(result.getSeconds()).toBe(0)
    expect(result.getMilliseconds()).toBe(0)
  })

  test("roundtrips with formatDateKey", () => {
    const original = "2024-06-20"
    const parsed = parseDateKey(original)
    const formatted = formatDateKey(parsed)
    expect(formatted).toBe(original)
  })
})

describe("startOfDay", () => {
  test("returns midnight", () => {
    const date = new Date(2024, 0, 15, 14, 30, 45, 123)
    const result = startOfDay(date)
    expect(result.getHours()).toBe(0)
    expect(result.getMinutes()).toBe(0)
    expect(result.getSeconds()).toBe(0)
    expect(result.getMilliseconds()).toBe(0)
  })

  test("preserves date", () => {
    const date = new Date(2024, 0, 15, 14, 30)
    const result = startOfDay(date)
    expect(result.getDate()).toBe(15)
    expect(result.getMonth()).toBe(0)
    expect(result.getFullYear()).toBe(2024)
  })

  test("does not mutate original", () => {
    const date = new Date(2024, 0, 15, 14, 30)
    const originalHours = date.getHours()
    startOfDay(date)
    expect(date.getHours()).toBe(originalHours)
  })
})

describe("getDayOfWeek", () => {
  test("returns 0 for Sunday", () => {
    const sunday = new Date(2024, 0, 14) // Jan 14, 2024 is Sunday
    expect(getDayOfWeek(sunday)).toBe(0)
  })

  test("returns 6 for Saturday", () => {
    const saturday = new Date(2024, 0, 13) // Jan 13, 2024 is Saturday
    expect(getDayOfWeek(saturday)).toBe(6)
  })
})

describe("getDayOfMonth", () => {
  test("returns day of month", () => {
    const date = new Date(2024, 0, 15)
    expect(getDayOfMonth(date)).toBe(15)
  })
})

describe("getMonth", () => {
  test("returns month index (0-11)", () => {
    const date = new Date(2024, 5, 15) // June
    expect(getMonth(date)).toBe(5)
  })
})

describe("getYear", () => {
  test("returns full year", () => {
    const date = new Date(2024, 0, 15)
    expect(getYear(date)).toBe(2024)
  })
})

describe("isToday", () => {
  test("returns true for today", () => {
    const today = new Date()
    expect(isToday(today)).toBe(true)
  })

  test("returns false for yesterday", () => {
    const yesterday = addDays(new Date(), -1)
    expect(isToday(yesterday)).toBe(false)
  })

  test("returns false for tomorrow", () => {
    const tomorrow = addDays(new Date(), 1)
    expect(isToday(tomorrow)).toBe(false)
  })
})

describe("getDateRange", () => {
  test("returns array of dates inclusive", () => {
    const start = new Date(2024, 0, 15)
    const end = new Date(2024, 0, 17)
    const result = getDateRange(start, end)
    expect(result.length).toBe(3)
    expect(formatDateKey(result[0])).toBe("2024-01-15")
    expect(formatDateKey(result[1])).toBe("2024-01-16")
    expect(formatDateKey(result[2])).toBe("2024-01-17")
  })

  test("returns single date when start equals end", () => {
    const date = new Date(2024, 0, 15)
    const result = getDateRange(date, date)
    expect(result.length).toBe(1)
  })

  test("returns empty for invalid range", () => {
    const start = new Date(2024, 0, 17)
    const end = new Date(2024, 0, 15)
    const result = getDateRange(start, end)
    expect(result.length).toBe(0)
  })
})

describe("getDatesAround", () => {
  test("returns dates centered on given date", () => {
    const center = new Date(2024, 0, 15)
    const result = getDatesAround(center, 2, 2)
    expect(result.length).toBe(5)
    expect(formatDateKey(result[0])).toBe("2024-01-13")
    expect(formatDateKey(result[2])).toBe("2024-01-15")
    expect(formatDateKey(result[4])).toBe("2024-01-17")
  })

  test("handles asymmetric before/after", () => {
    const center = new Date(2024, 0, 15)
    const result = getDatesAround(center, 1, 3)
    expect(result.length).toBe(5)
    expect(formatDateKey(result[0])).toBe("2024-01-14")
    expect(formatDateKey(result[4])).toBe("2024-01-18")
  })
})
