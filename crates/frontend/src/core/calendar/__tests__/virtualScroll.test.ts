import { describe, expect, test } from "bun:test"
import {
  calculateDayIndex,
  calculateDayWidth,
  calculateHighlightedDay,
  calculateRecenterOffset,
  calculateScrollPosition,
  calculateTotalWidth,
  calculateVirtualWindow,
  calculateVisibleDays,
  calculateWindowDates,
  calculateWindowStartDate,
  DEFAULT_VIRTUAL_SCROLL_CONFIG,
  isSameCalendarDay,
  shouldRecenter,
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
  const windowStart = new Date("2025-01-01")

  test("centers single day (visibleDays=1)", () => {
    const target = new Date("2025-01-10") // Day index 9
    const result = calculateScrollPosition(target, windowStart, 400, 400, 1)
    // floor(1/2) = 0 days before, first visible = 9
    expect(result).toBe(3600) // 9 * 400
  })

  test("centers group of 2 days", () => {
    const target = new Date("2025-01-10") // Day index 9
    const result = calculateScrollPosition(target, windowStart, 300, 600, 2)
    // floor(2/2) = 1 day before, first visible = 8
    expect(result).toBe(2400) // 8 * 300
  })

  test("centers group of 3 days", () => {
    const target = new Date("2025-01-10") // Day index 9
    const result = calculateScrollPosition(target, windowStart, 300, 900, 3)
    // floor(3/2) = 1 day before, first visible = 8
    expect(result).toBe(2400) // 8 * 300
  })

  test("centers group of 5 days", () => {
    const target = new Date("2025-01-10") // Day index 9
    const result = calculateScrollPosition(target, windowStart, 250, 1250, 5)
    // floor(5/2) = 2 days before, first visible = 7
    expect(result).toBe(1750) // 7 * 250
  })

  test("centers group of 7 days", () => {
    const target = new Date("2025-01-10") // Day index 9
    const result = calculateScrollPosition(target, windowStart, 250, 1750, 7)
    // floor(7/2) = 3 days before, first visible = 6
    expect(result).toBe(1500) // 6 * 250
  })

  test("clamps to 0 for early dates", () => {
    const target = new Date("2025-01-01") // Day index 0
    const result = calculateScrollPosition(target, windowStart, 300, 900, 3)
    // Would be -1 * 300 = -300, clamped to 0
    expect(result).toBe(0)
  })

  test("does not return negative values", () => {
    const target = new Date("2025-01-02") // Day index 1
    const result = calculateScrollPosition(target, windowStart, 300, 900, 3)
    // floor(3/2) = 1 day before, first visible = 0
    expect(result).toBeGreaterThanOrEqual(0)
    expect(result).toBe(0) // 0 * 300
  })

  test("centers day when dayWidth is 75% of container (500-749px special case)", () => {
    const target = new Date("2025-01-10") // Day index 9
    const containerWidth = 600
    const dayWidth = 450 // 75% of 600
    const visibleDays = 1
    const result = calculateScrollPosition(
      target,
      windowStart,
      dayWidth,
      containerWidth,
      visibleDays,
    )

    // For 75% width:
    // - dayWidth = 450, containerWidth = 600
    // - expectedVisibleWidth = 450 (< 600)
    // - centerOffset = (600 - 450) / 2 = 75
    // - scrollLeft = 9 * 450 - 75 = 4050 - 75 = 3975
    expect(result).toBe(3975)
  })

  test("centers day for any partial-viewport case", () => {
    const target = new Date("2025-01-10") // Day index 9
    const containerWidth = 1000
    const dayWidth = 800 // 80% of container
    const visibleDays = 1
    const result = calculateScrollPosition(
      target,
      windowStart,
      dayWidth,
      containerWidth,
      visibleDays,
    )

    // centerOffset = (1000 - 800) / 2 = 100
    // scrollLeft = 9 * 800 - 100 = 7100
    expect(result).toBe(7100)
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
  test("returns 1 for containerWidth <= 0", () => {
    expect(calculateVisibleDays(0)).toBe(1)
    expect(calculateVisibleDays(-100)).toBe(1)
  })

  test("returns 1 for viewport < 500px (full width)", () => {
    expect(calculateVisibleDays(320)).toBe(1)
    expect(calculateVisibleDays(375)).toBe(1)
    expect(calculateVisibleDays(400)).toBe(1)
    expect(calculateVisibleDays(499)).toBe(1)
  })

  test("returns 1 for viewport 500px - 749px (75% width special case)", () => {
    expect(calculateVisibleDays(500)).toBe(1)
    expect(calculateVisibleDays(600)).toBe(1)
    expect(calculateVisibleDays(700)).toBe(1)
    expect(calculateVisibleDays(749)).toBe(1)
  })

  test("returns 3 for viewport 750px - 1249px", () => {
    expect(calculateVisibleDays(750)).toBe(3)
    expect(calculateVisibleDays(900)).toBe(3)
    expect(calculateVisibleDays(1000)).toBe(3)
    expect(calculateVisibleDays(1249)).toBe(3)
  })

  test("returns 5 for viewport 1250px - 1749px", () => {
    expect(calculateVisibleDays(1250)).toBe(5)
    expect(calculateVisibleDays(1400)).toBe(5)
    expect(calculateVisibleDays(1500)).toBe(5)
    expect(calculateVisibleDays(1749)).toBe(5)
  })

  test("returns 7 for viewport >= 1750px", () => {
    expect(calculateVisibleDays(1750)).toBe(7)
    expect(calculateVisibleDays(1920)).toBe(7)
    expect(calculateVisibleDays(2560)).toBe(7)
  })

  test("boundary values", () => {
    // At breakpoint boundaries
    expect(calculateVisibleDays(499)).toBe(1)
    expect(calculateVisibleDays(500)).toBe(1) // 75% width special case
    expect(calculateVisibleDays(749)).toBe(1) // 75% width special case
    expect(calculateVisibleDays(750)).toBe(3)
    expect(calculateVisibleDays(1249)).toBe(3)
    expect(calculateVisibleDays(1250)).toBe(5)
    expect(calculateVisibleDays(1749)).toBe(5)
    expect(calculateVisibleDays(1750)).toBe(7)
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
    expect(calculateDayWidth(375, 1)).toBe(375) // < 500px: full width
  })

  test("returns 75% width for 500-749px viewport with 1 visible day", () => {
    // Special case: 500-749px viewport uses 75% width for better appearance
    expect(calculateDayWidth(500, 1)).toBe(375) // 500 * 0.75
    expect(calculateDayWidth(600, 1)).toBe(450) // 600 * 0.75
    expect(calculateDayWidth(700, 1)).toBe(525) // 700 * 0.75
    expect(calculateDayWidth(749, 1)).toBe(561.75) // 749 * 0.75
  })

  test("returns full width for < 500px viewport with 1 visible day", () => {
    // Below 500px uses full width, not 75%
    expect(calculateDayWidth(400, 1)).toBe(400)
    expect(calculateDayWidth(499, 1)).toBe(499)
  })

  test("returns normal division for >= 750px viewport", () => {
    // At 750px and above, normal division applies
    expect(calculateDayWidth(750, 3)).toBe(250)
    expect(calculateDayWidth(900, 3)).toBe(300)
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
