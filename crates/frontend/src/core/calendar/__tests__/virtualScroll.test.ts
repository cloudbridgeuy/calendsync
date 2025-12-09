import { describe, expect, test } from "bun:test"
import {
  calculateDayIndex,
  calculateDayWidth,
  calculateHighlightedDay,
  calculateRecenterOffset,
  calculateScrollPosition,
  calculateSnapScrollPosition,
  calculateTotalWidth,
  calculateVirtualWindow,
  calculateVisibleDays,
  calculateWindowDates,
  calculateWindowStartDate,
  DEFAULT_VIRTUAL_SCROLL_CONFIG,
  isSameCalendarDay,
  shouldRecenter,
  shouldSnapToDay,
} from "../virtualScroll"

describe("calculateVirtualWindow", () => {
  test("returns correct number of dates based on window size", () => {
    const center = new Date("2025-01-15")
    const dates = calculateVirtualWindow(center, DEFAULT_VIRTUAL_SCROLL_CONFIG)

    expect(dates).toHaveLength(21)
  })

  test("centers the window on the given date", () => {
    const center = new Date("2025-01-15")
    const config = { windowSize: 21, bufferDays: 10, recenterThreshold: 3 }
    const dates = calculateVirtualWindow(center, config)

    // Center date should be at index 10 (bufferDays)
    expect(dates[10].toISOString().slice(0, 10)).toBe("2025-01-15")
  })

  test("first date is bufferDays before center", () => {
    const center = new Date("2025-01-15")
    const config = { windowSize: 21, bufferDays: 10, recenterThreshold: 3 }
    const dates = calculateVirtualWindow(center, config)

    // First date should be Jan 5 (10 days before Jan 15)
    expect(dates[0].toISOString().slice(0, 10)).toBe("2025-01-05")
  })

  test("last date is (windowSize - bufferDays - 1) days after center", () => {
    const center = new Date("2025-01-15")
    const config = { windowSize: 21, bufferDays: 10, recenterThreshold: 3 }
    const dates = calculateVirtualWindow(center, config)

    // Last date should be Jan 25 (10 days after Jan 15)
    expect(dates[20].toISOString().slice(0, 10)).toBe("2025-01-25")
  })

  test("works with custom config", () => {
    const center = new Date("2025-06-01")
    const config = { windowSize: 7, bufferDays: 3, recenterThreshold: 1 }
    const dates = calculateVirtualWindow(center, config)

    expect(dates).toHaveLength(7)
    expect(dates[3].toISOString().slice(0, 10)).toBe("2025-06-01")
    expect(dates[0].toISOString().slice(0, 10)).toBe("2025-05-29")
    expect(dates[6].toISOString().slice(0, 10)).toBe("2025-06-04")
  })

  test("handles year boundary", () => {
    const center = new Date("2025-01-02")
    const config = { windowSize: 7, bufferDays: 3, recenterThreshold: 1 }
    const dates = calculateVirtualWindow(center, config)

    expect(dates[0].toISOString().slice(0, 10)).toBe("2024-12-30")
    expect(dates[3].toISOString().slice(0, 10)).toBe("2025-01-02")
  })
})

describe("calculateScrollPosition", () => {
  test("returns 0 when target is at window start with small container", () => {
    const windowStart = new Date("2025-01-05")
    const target = new Date("2025-01-05")
    const result = calculateScrollPosition(target, windowStart, 100, 100)

    expect(result).toBe(0)
  })

  test("centers the target date in viewport", () => {
    const windowStart = new Date("2025-01-05")
    const target = new Date("2025-01-15") // 10 days after start
    const dayWidth = 100
    const containerWidth = 700

    const result = calculateScrollPosition(target, windowStart, dayWidth, containerWidth)

    // Target is at 1000px (10 * 100)
    // Center offset is 350px - 50px = 300px (half container - half day)
    // Expected: 1000 - 300 = 700px
    expect(result).toBe(700)
  })

  test("does not return negative values", () => {
    const windowStart = new Date("2025-01-05")
    const target = new Date("2025-01-05") // At start
    const dayWidth = 100
    const containerWidth = 700

    const result = calculateScrollPosition(target, windowStart, dayWidth, containerWidth)

    expect(result).toBeGreaterThanOrEqual(0)
  })

  test("handles single day width viewport", () => {
    const windowStart = new Date("2025-01-05")
    const target = new Date("2025-01-10") // 5 days after start
    const dayWidth = 200
    const containerWidth = 200

    const result = calculateScrollPosition(target, windowStart, dayWidth, containerWidth)

    // Target at 1000px, center offset = 100 - 100 = 0
    // Expected: 1000 - 0 = 1000px
    expect(result).toBe(1000)
  })
})

describe("calculateHighlightedDay", () => {
  test("returns window start date when scrolled to beginning", () => {
    const windowStart = new Date("2025-01-05")
    const dayWidth = 100
    const containerWidth = 300

    const result = calculateHighlightedDay(0, containerWidth, dayWidth, windowStart)

    // Center of viewport is at 150px, which is in day index 1
    expect(result.toISOString().slice(0, 10)).toBe("2025-01-06")
  })

  test("calculates correct day at scroll position", () => {
    const windowStart = new Date("2025-01-05")
    const dayWidth = 100
    const containerWidth = 300
    const scrollLeft = 500

    const result = calculateHighlightedDay(scrollLeft, containerWidth, dayWidth, windowStart)

    // Center is at 500 + 150 = 650px, which is day index 6
    expect(result.toISOString().slice(0, 10)).toBe("2025-01-11") // Jan 5 + 6 days
  })

  test("handles exact day boundaries", () => {
    const windowStart = new Date("2025-01-05")
    const dayWidth = 100
    const containerWidth = 100

    // Scroll to exactly align day 5 in center
    const scrollLeft = 500

    const result = calculateHighlightedDay(scrollLeft, containerWidth, dayWidth, windowStart)

    // Center at 550px, day index 5
    expect(result.toISOString().slice(0, 10)).toBe("2025-01-10")
  })
})

describe("shouldRecenter", () => {
  const dayWidth = 100
  const windowSize = 21
  const totalWidth = windowSize * dayWidth // 2100
  const containerWidth = 700
  const threshold = 3

  test("returns 'start' when near beginning", () => {
    const scrollLeft = 200 // Within 300px (3 days) of start

    const result = shouldRecenter(scrollLeft, totalWidth, containerWidth, dayWidth, threshold)

    expect(result).toBe("start")
  })

  test("returns 'end' when near end", () => {
    const maxScroll = totalWidth - containerWidth // 1400
    const scrollLeft = maxScroll - 200 // Within 300px of end

    const result = shouldRecenter(scrollLeft, totalWidth, containerWidth, dayWidth, threshold)

    expect(result).toBe("end")
  })

  test("returns null when in safe zone", () => {
    const scrollLeft = 700 // Middle of scroll range

    const result = shouldRecenter(scrollLeft, totalWidth, containerWidth, dayWidth, threshold)

    expect(result).toBeNull()
  })

  test("returns 'start' at exactly threshold boundary", () => {
    const scrollLeft = 299 // Just under threshold

    const result = shouldRecenter(scrollLeft, totalWidth, containerWidth, dayWidth, threshold)

    expect(result).toBe("start")
  })

  test("returns null at exactly threshold + 1", () => {
    const scrollLeft = 300 // At threshold

    const result = shouldRecenter(scrollLeft, totalWidth, containerWidth, dayWidth, threshold)

    expect(result).toBeNull()
  })
})

describe("calculateRecenterOffset", () => {
  const dayWidth = 100
  const shiftDays = 6

  test("shifts window forward when direction is 'end'", () => {
    const windowStart = new Date("2025-01-05")

    const result = calculateRecenterOffset("end", windowStart, dayWidth, shiftDays)

    expect(result.newWindowStartDate.toISOString().slice(0, 10)).toBe("2025-01-11")
    expect(result.scrollAdjustment).toBe(-600) // Scroll back 6 days worth
  })

  test("shifts window backward when direction is 'start'", () => {
    const windowStart = new Date("2025-01-15")

    const result = calculateRecenterOffset("start", windowStart, dayWidth, shiftDays)

    expect(result.newWindowStartDate.toISOString().slice(0, 10)).toBe("2025-01-09")
    expect(result.scrollAdjustment).toBe(600) // Scroll forward 6 days worth
  })

  test("scroll adjustment is proportional to shift and day width", () => {
    const windowStart = new Date("2025-01-10")
    const largeDayWidth = 200
    const smallShift = 3

    const result = calculateRecenterOffset("end", windowStart, largeDayWidth, smallShift)

    expect(result.scrollAdjustment).toBe(-600) // 3 * 200 = 600, negative for end
  })
})

describe("isSameCalendarDay", () => {
  test("returns true for same date different times", () => {
    const a = new Date("2025-01-15T10:30:00")
    const b = new Date("2025-01-15T22:45:00")

    expect(isSameCalendarDay(a, b)).toBe(true)
  })

  test("returns false for different dates", () => {
    const a = new Date("2025-01-15")
    const b = new Date("2025-01-16")

    expect(isSameCalendarDay(a, b)).toBe(false)
  })

  test("returns true for identical dates", () => {
    const a = new Date("2025-01-15")
    const b = new Date("2025-01-15")

    expect(isSameCalendarDay(a, b)).toBe(true)
  })

  test("handles midnight boundary", () => {
    const a = new Date("2025-01-15T23:59:59")
    const b = new Date("2025-01-16T00:00:01")

    expect(isSameCalendarDay(a, b)).toBe(false)
  })
})

describe("calculateWindowStartDate", () => {
  test("returns correct start date", () => {
    const center = new Date("2025-01-15")
    const bufferDays = 10

    const result = calculateWindowStartDate(center, bufferDays)

    expect(result.toISOString().slice(0, 10)).toBe("2025-01-05")
  })

  test("handles month boundary", () => {
    const center = new Date("2025-02-05")
    const bufferDays = 10

    const result = calculateWindowStartDate(center, bufferDays)

    expect(result.toISOString().slice(0, 10)).toBe("2025-01-26")
  })
})

describe("calculateTotalWidth", () => {
  test("calculates correct total width", () => {
    expect(calculateTotalWidth(21, 100)).toBe(2100)
    expect(calculateTotalWidth(7, 200)).toBe(1400)
    expect(calculateTotalWidth(1, 500)).toBe(500)
  })
})

describe("calculateDayIndex", () => {
  test("returns 0 for window start date", () => {
    const windowStart = new Date("2025-01-05")
    const date = new Date("2025-01-05")

    expect(calculateDayIndex(date, windowStart)).toBe(0)
  })

  test("returns correct index for later dates", () => {
    const windowStart = new Date("2025-01-05")
    const date = new Date("2025-01-15")

    expect(calculateDayIndex(date, windowStart)).toBe(10)
  })

  test("handles dates with same time components", () => {
    // Both dates should have same time to get accurate day index
    const windowStart = new Date(2025, 0, 5, 12, 0, 0) // Jan 5, 2025 at noon
    const date = new Date(2025, 0, 7, 12, 0, 0) // Jan 7, 2025 at noon

    expect(calculateDayIndex(date, windowStart)).toBe(2)
  })
})

describe("calculateVisibleDays", () => {
  test("returns 3 for containerWidth <= 0", () => {
    expect(calculateVisibleDays(0)).toBe(3)
    expect(calculateVisibleDays(-100)).toBe(3)
  })

  test("returns 1 for mobile portrait (< 480px)", () => {
    expect(calculateVisibleDays(320)).toBe(1)
    expect(calculateVisibleDays(375)).toBe(1)
    expect(calculateVisibleDays(400)).toBe(1)
    expect(calculateVisibleDays(479)).toBe(1)
  })

  test("returns 3 for mobile landscape (480-767px)", () => {
    expect(calculateVisibleDays(480)).toBe(3)
    expect(calculateVisibleDays(640)).toBe(3)
    expect(calculateVisibleDays(767)).toBe(3)
  })

  test("returns 5 for tablet (768-1023px)", () => {
    expect(calculateVisibleDays(768)).toBe(5)
    expect(calculateVisibleDays(800)).toBe(5)
    expect(calculateVisibleDays(1023)).toBe(5)
  })

  test("returns 5 for desktop (1024-1439px)", () => {
    expect(calculateVisibleDays(1024)).toBe(5)
    expect(calculateVisibleDays(1200)).toBe(5)
    expect(calculateVisibleDays(1439)).toBe(5)
  })

  test("returns 7 for large desktop (>= 1440px)", () => {
    expect(calculateVisibleDays(1440)).toBe(7)
    expect(calculateVisibleDays(1920)).toBe(7)
    expect(calculateVisibleDays(2560)).toBe(7)
  })

  test("test boundary values", () => {
    // Test exact boundary values
    expect(calculateVisibleDays(479)).toBe(1) // Just under 480
    expect(calculateVisibleDays(480)).toBe(3) // At 480
    expect(calculateVisibleDays(767)).toBe(3) // Just under 768
    expect(calculateVisibleDays(768)).toBe(5) // At 768
    expect(calculateVisibleDays(1023)).toBe(5) // Just under 1024
    expect(calculateVisibleDays(1024)).toBe(5) // At 1024
    expect(calculateVisibleDays(1439)).toBe(5) // Just under 1440
    expect(calculateVisibleDays(1440)).toBe(7) // At 1440
  })
})

describe("calculateDayWidth", () => {
  test("returns 100 for containerWidth <= 0", () => {
    expect(calculateDayWidth(0, 5)).toBe(100)
    expect(calculateDayWidth(-100, 5)).toBe(100)
  })

  test("returns 100 for visibleDays <= 0", () => {
    expect(calculateDayWidth(700, 0)).toBe(100)
    expect(calculateDayWidth(700, -5)).toBe(100)
  })

  test("returns 100 for both invalid inputs", () => {
    expect(calculateDayWidth(0, 0)).toBe(100)
    expect(calculateDayWidth(-100, -5)).toBe(100)
  })

  test("returns containerWidth / visibleDays for valid inputs", () => {
    expect(calculateDayWidth(700, 7)).toBe(100)
    expect(calculateDayWidth(1500, 5)).toBe(300)
    expect(calculateDayWidth(375, 1)).toBe(375)
  })

  test("example: calculateDayWidth(1440, 5) = 288", () => {
    expect(calculateDayWidth(1440, 5)).toBe(288)
  })

  test("example: calculateDayWidth(768, 3) = 256", () => {
    expect(calculateDayWidth(768, 3)).toBe(256)
  })

  test("handles decimal results", () => {
    // Should return precise division result
    expect(calculateDayWidth(1000, 3)).toBeCloseTo(333.33, 2)
    expect(calculateDayWidth(800, 7)).toBeCloseTo(114.29, 2)
  })
})

describe("calculateWindowDates", () => {
  test("returns correct number of dates", () => {
    const startDate = new Date("2025-01-05")
    const dates = calculateWindowDates(startDate, 21)

    expect(dates).toHaveLength(21)
  })

  test("returns consecutive days starting from startDate", () => {
    const startDate = new Date("2025-01-05")
    const dates = calculateWindowDates(startDate, 5)

    expect(dates[0].toISOString().slice(0, 10)).toBe("2025-01-05")
    expect(dates[1].toISOString().slice(0, 10)).toBe("2025-01-06")
    expect(dates[2].toISOString().slice(0, 10)).toBe("2025-01-07")
    expect(dates[3].toISOString().slice(0, 10)).toBe("2025-01-08")
    expect(dates[4].toISOString().slice(0, 10)).toBe("2025-01-09")
  })

  test("returns single date for windowSize of 1", () => {
    const startDate = new Date("2025-06-15")
    const dates = calculateWindowDates(startDate, 1)

    expect(dates).toHaveLength(1)
    expect(dates[0].toISOString().slice(0, 10)).toBe("2025-06-15")
  })

  test("returns empty array for windowSize of 0", () => {
    const startDate = new Date("2025-01-01")
    const dates = calculateWindowDates(startDate, 0)

    expect(dates).toHaveLength(0)
  })

  test("returns empty array for negative windowSize", () => {
    const startDate = new Date("2025-01-01")
    const dates = calculateWindowDates(startDate, -5)

    expect(dates).toHaveLength(0)
  })

  test("handles month boundary", () => {
    const startDate = new Date("2025-01-30")
    const dates = calculateWindowDates(startDate, 5)

    expect(dates[0].toISOString().slice(0, 10)).toBe("2025-01-30")
    expect(dates[1].toISOString().slice(0, 10)).toBe("2025-01-31")
    expect(dates[2].toISOString().slice(0, 10)).toBe("2025-02-01")
    expect(dates[3].toISOString().slice(0, 10)).toBe("2025-02-02")
    expect(dates[4].toISOString().slice(0, 10)).toBe("2025-02-03")
  })

  test("handles year boundary", () => {
    const startDate = new Date("2024-12-30")
    const dates = calculateWindowDates(startDate, 5)

    expect(dates[0].toISOString().slice(0, 10)).toBe("2024-12-30")
    expect(dates[1].toISOString().slice(0, 10)).toBe("2024-12-31")
    expect(dates[2].toISOString().slice(0, 10)).toBe("2025-01-01")
    expect(dates[3].toISOString().slice(0, 10)).toBe("2025-01-02")
    expect(dates[4].toISOString().slice(0, 10)).toBe("2025-01-03")
  })

  test("matches calculateVirtualWindow behavior when centered", () => {
    // calculateVirtualWindow centers on a date
    // calculateWindowDates starts from a given date
    // If we call calculateWindowDates with the start date from calculateVirtualWindow, they should match
    const centerDate = new Date("2025-01-15")
    const config = { windowSize: 21, bufferDays: 10, recenterThreshold: 3 }

    const virtualWindowDates = calculateVirtualWindow(centerDate, config)
    const startDate = calculateWindowStartDate(centerDate, config.bufferDays)
    const windowDates = calculateWindowDates(startDate, config.windowSize)

    expect(windowDates).toHaveLength(virtualWindowDates.length)
    for (let i = 0; i < windowDates.length; i++) {
      expect(windowDates[i].toISOString().slice(0, 10)).toBe(
        virtualWindowDates[i].toISOString().slice(0, 10),
      )
    }
  })
})

describe("shouldSnapToDay", () => {
  test("returns true when visibleDays is 1", () => {
    expect(shouldSnapToDay(1)).toBe(true)
  })

  test("returns false when visibleDays is greater than 1", () => {
    expect(shouldSnapToDay(3)).toBe(false)
    expect(shouldSnapToDay(5)).toBe(false)
    expect(shouldSnapToDay(7)).toBe(false)
  })

  test("returns false for zero or negative visibleDays", () => {
    expect(shouldSnapToDay(0)).toBe(false)
    expect(shouldSnapToDay(-1)).toBe(false)
  })
})

describe("calculateSnapScrollPosition", () => {
  const dayWidth = 375 // typical mobile width
  const containerWidth = 375
  const windowStart = new Date("2025-01-05")

  test("snaps to current day when scroll is less than half a day", () => {
    const result = calculateSnapScrollPosition(100, dayWidth, containerWidth, windowStart)
    expect(result.targetDate.toISOString().slice(0, 10)).toBe("2025-01-05")
    expect(result.scrollPosition).toBe(0)
  })

  test("snaps to next day when scroll is more than half a day", () => {
    const result = calculateSnapScrollPosition(200, dayWidth, containerWidth, windowStart)
    expect(result.targetDate.toISOString().slice(0, 10)).toBe("2025-01-06")
  })

  test("snaps to exact day boundary when perfectly aligned", () => {
    const result = calculateSnapScrollPosition(375, dayWidth, containerWidth, windowStart)
    expect(result.targetDate.toISOString().slice(0, 10)).toBe("2025-01-06")
  })

  test("handles scroll position at start", () => {
    const result = calculateSnapScrollPosition(0, dayWidth, containerWidth, windowStart)
    expect(result.targetDate.toISOString().slice(0, 10)).toBe("2025-01-05")
    expect(result.scrollPosition).toBe(0)
  })

  test("handles scroll several days in", () => {
    const result = calculateSnapScrollPosition(1500, dayWidth, containerWidth, windowStart)
    // 1500 / 375 = 4, so should snap to day index 4
    expect(result.targetDate.toISOString().slice(0, 10)).toBe("2025-01-09")
  })
})
