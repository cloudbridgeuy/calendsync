import { describe, expect, test } from "bun:test"
import {
  calculateNowPositionPercent,
  calculateScrollToCurrentTime,
  findTodayColumnIndex,
  formatNowLabel,
} from "../nowIndicator"

describe("calculateNowPositionPercent", () => {
  test("midnight is 0%", () => {
    expect(calculateNowPositionPercent(0, 0)).toBe(0)
  })

  test("noon is 50%", () => {
    expect(calculateNowPositionPercent(12, 0)).toBe(50)
  })

  test("6 AM is 25%", () => {
    expect(calculateNowPositionPercent(6, 0)).toBe(25)
  })

  test("6:30 PM is 77.083...%", () => {
    // (18*60+30) / 1440 * 100 = 1110/1440*100 = 77.0833...
    expect(calculateNowPositionPercent(18, 30)).toBeCloseTo(77.0833, 2)
  })

  test("23:59 is near 100%", () => {
    expect(calculateNowPositionPercent(23, 59)).toBeCloseTo(99.9306, 2)
  })
})

describe("findTodayColumnIndex", () => {
  const today = new Date(2024, 5, 15) // June 15, 2024

  test("returns index when today is in rendered dates", () => {
    const dates = [new Date(2024, 5, 14), new Date(2024, 5, 15), new Date(2024, 5, 16)]
    expect(findTodayColumnIndex(dates, today)).toBe(1)
  })

  test("returns null when today is not in rendered dates", () => {
    const dates = [new Date(2024, 5, 10), new Date(2024, 5, 11)]
    expect(findTodayColumnIndex(dates, today)).toBeNull()
  })

  test("returns 0 when today is the first date", () => {
    const dates = [new Date(2024, 5, 15), new Date(2024, 5, 16)]
    expect(findTodayColumnIndex(dates, today)).toBe(0)
  })

  test("handles empty array", () => {
    expect(findTodayColumnIndex([], today)).toBeNull()
  })
})

describe("formatNowLabel", () => {
  test("formats midnight as 12:00 AM", () => {
    expect(formatNowLabel(0, 0)).toBe("12:00 AM")
  })

  test("formats noon as 12:00 PM", () => {
    expect(formatNowLabel(12, 0)).toBe("12:00 PM")
  })

  test("formats morning time", () => {
    expect(formatNowLabel(9, 5)).toBe("9:05 AM")
  })

  test("formats afternoon time", () => {
    expect(formatNowLabel(18, 38)).toBe("6:38 PM")
  })

  test("formats 11:59 PM", () => {
    expect(formatNowLabel(23, 59)).toBe("11:59 PM")
  })

  test("pads single-digit minutes", () => {
    expect(formatNowLabel(3, 7)).toBe("3:07 AM")
  })
})

describe("calculateScrollToCurrentTime", () => {
  const viewportHeight = 600
  const totalHeight = 1440 // 24 * 60

  test("places current time in upper third", () => {
    // 12:00 = 720px into 1440px total
    const scroll = calculateScrollToCurrentTime(12, 0, viewportHeight, totalHeight)
    // target = 720 - 600/3 = 720 - 200 = 520
    expect(scroll).toBe(520)
  })

  test("clamps to 0 for early morning", () => {
    // 1:00 = 60px, target = 60 - 200 = -140 → clamped to 0
    const scroll = calculateScrollToCurrentTime(1, 0, viewportHeight, totalHeight)
    expect(scroll).toBe(0)
  })

  test("clamps to max for late night", () => {
    // 23:50 = 1430px, target = 1430 - 200 = 1230, max = 1440 - 600 = 840
    const scroll = calculateScrollToCurrentTime(23, 50, viewportHeight, totalHeight)
    expect(scroll).toBe(840)
  })

  test("midnight returns 0", () => {
    const scroll = calculateScrollToCurrentTime(0, 0, viewportHeight, totalHeight)
    expect(scroll).toBe(0)
  })
})
