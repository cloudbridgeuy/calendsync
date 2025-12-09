import { describe, expect, test } from "bun:test"
import {
  calculateCenterDayIndex,
  calculateCenteredScrollPosition,
  detectEdgeProximity,
  isScrollable,
} from "../navigation"

describe("isScrollable", () => {
  test("returns true when scrollWidth > clientWidth", () => {
    expect(isScrollable(1000, 500)).toBe(true)
    expect(isScrollable(2100, 700)).toBe(true)
    expect(isScrollable(101, 100)).toBe(true)
  })

  test("returns false when scrollWidth equals clientWidth", () => {
    expect(isScrollable(500, 500)).toBe(false)
    expect(isScrollable(700, 700)).toBe(false)
    expect(isScrollable(0, 0)).toBe(false)
  })

  test("returns false when scrollWidth < clientWidth", () => {
    expect(isScrollable(500, 1000)).toBe(false)
    expect(isScrollable(700, 2100)).toBe(false)
    expect(isScrollable(99, 100)).toBe(false)
  })

  test("handles edge case with 1px difference", () => {
    expect(isScrollable(501, 500)).toBe(true)
    expect(isScrollable(500, 501)).toBe(false)
  })

  test("handles zero values", () => {
    expect(isScrollable(0, 100)).toBe(false)
    expect(isScrollable(100, 0)).toBe(true)
  })

  test("handles negative values gracefully", () => {
    // Negative values shouldn't occur in practice, but test behavior
    expect(isScrollable(-100, 100)).toBe(false)
    expect(isScrollable(100, -100)).toBe(true)
  })
})

describe("calculateCenteredScrollPosition", () => {
  test("returns null when content is not scrollable (totalContentWidth <= containerWidth)", () => {
    // Total content width equals container width
    expect(calculateCenteredScrollPosition(5, 100, 700, 700)).toBeNull()

    // Total content width less than container width
    expect(calculateCenteredScrollPosition(5, 100, 700, 500)).toBeNull()
  })

  test("centers target day in viewport", () => {
    // Day 10 with dayWidth=100, containerWidth=700, totalWidth=2100
    // Target left edge: 10 * 100 = 1000px
    // Center offset: (700 - 100) / 2 = 300px
    // Expected scroll: 1000 - 300 = 700px
    const result = calculateCenteredScrollPosition(10, 100, 700, 2100)
    expect(result).toBe(700)
  })

  test("clamps to 0 when centering would scroll before start", () => {
    // Day 1 with large container
    // Target left: 1 * 100 = 100px
    // Center offset: (700 - 100) / 2 = 300px
    // Desired scroll: 100 - 300 = -200px
    // Clamped to: 0
    const result = calculateCenteredScrollPosition(1, 100, 700, 2100)
    expect(result).toBe(0)
  })

  test("clamps to maxScroll when centering would scroll past end", () => {
    // Day 20 with dayWidth=100, containerWidth=700, totalWidth=2100
    // Target left: 20 * 100 = 2000px
    // Center offset: (700 - 100) / 2 = 300px
    // Desired scroll: 2000 - 300 = 1700px
    // maxScroll: 2100 - 700 = 1400px
    // Clamped to: 1400px
    const result = calculateCenteredScrollPosition(20, 100, 700, 2100)
    expect(result).toBe(1400)
  })

  test("handles day 0 (first day)", () => {
    // Day 0 should result in scroll position 0 (after clamping)
    const result = calculateCenteredScrollPosition(0, 100, 700, 2100)
    expect(result).toBe(0)
  })

  test("handles middle of scroll range", () => {
    // Day 7 with dayWidth=100, containerWidth=700, totalWidth=2100
    // Target left: 7 * 100 = 700px
    // Center offset: (700 - 100) / 2 = 300px
    // Expected scroll: 700 - 300 = 400px
    const result = calculateCenteredScrollPosition(7, 100, 700, 2100)
    expect(result).toBe(400)
  })

  test("works with different dayWidth values", () => {
    // Day 5 with dayWidth=200, containerWidth=800, totalWidth=3000
    // Target left: 5 * 200 = 1000px
    // Center offset: (800 - 200) / 2 = 300px
    // Expected scroll: 1000 - 300 = 700px
    const result = calculateCenteredScrollPosition(5, 200, 800, 3000)
    expect(result).toBe(700)
  })

  test("works with single day visible (containerWidth = dayWidth)", () => {
    // Day 5 with dayWidth=200, containerWidth=200, totalWidth=2100
    // Target left: 5 * 200 = 1000px
    // Center offset: (200 - 200) / 2 = 0px
    // Expected scroll: 1000 - 0 = 1000px
    const result = calculateCenteredScrollPosition(5, 200, 200, 2100)
    expect(result).toBe(1000)
  })

  test("returns exact maxScroll at last position", () => {
    // Day 20 in 21-day window (indexes 0-20)
    // dayWidth=100, containerWidth=700, totalWidth=2100
    // maxScroll = 2100 - 700 = 1400
    const result = calculateCenteredScrollPosition(20, 100, 700, 2100)
    expect(result).toBe(1400)
  })

  test("handles fractional pixel values", () => {
    // Day 5 with dayWidth=150, containerWidth=500, totalWidth=2250
    // Target left: 5 * 150 = 750px
    // Center offset: (500 - 150) / 2 = 175px
    // Expected scroll: 750 - 175 = 575px
    const result = calculateCenteredScrollPosition(5, 150, 500, 2250)
    expect(result).toBe(575)
  })

  test("ensures returned position is within valid range [0, maxScroll]", () => {
    const dayWidth = 100
    const containerWidth = 700
    const totalWidth = 2100
    const maxScroll = totalWidth - containerWidth // 1400

    // Test various day indexes
    for (let dayIndex = 0; dayIndex <= 20; dayIndex++) {
      const result = calculateCenteredScrollPosition(dayIndex, dayWidth, containerWidth, totalWidth)
      expect(result).not.toBeNull()
      expect(result).toBeGreaterThanOrEqual(0)
      expect(result).toBeLessThanOrEqual(maxScroll)
    }
  })
})

describe("calculateCenterDayIndex", () => {
  test("returns correct index when scrolled to beginning", () => {
    // scrollLeft=0, containerWidth=700, dayWidth=100
    // Center position: 0 + 700/2 = 350px
    // Day index: floor(350 / 100) = 3
    const result = calculateCenterDayIndex(0, 700, 100)
    expect(result).toBe(3)
  })

  test("returns correct index at various scroll positions", () => {
    const containerWidth = 700
    const dayWidth = 100

    // scrollLeft=200: center at 200 + 350 = 550px, day 5
    expect(calculateCenterDayIndex(200, containerWidth, dayWidth)).toBe(5)

    // scrollLeft=500: center at 500 + 350 = 850px, day 8
    expect(calculateCenterDayIndex(500, containerWidth, dayWidth)).toBe(8)

    // scrollLeft=1000: center at 1000 + 350 = 1350px, day 13
    expect(calculateCenterDayIndex(1000, containerWidth, dayWidth)).toBe(13)
  })

  test("handles exact day boundaries", () => {
    const containerWidth = 700
    const dayWidth = 100

    // Center exactly on day boundary (day 10 starts at 1000px)
    // scrollLeft=650: center at 650 + 350 = 1000px (day 10 boundary)
    expect(calculateCenterDayIndex(650, containerWidth, dayWidth)).toBe(10)
  })

  test("handles fractional results (floors to integer)", () => {
    // scrollLeft=100, containerWidth=700, dayWidth=100
    // Center position: 100 + 350 = 450px
    // Day index: floor(450 / 100) = 4
    expect(calculateCenterDayIndex(100, 700, 100)).toBe(4)
  })

  test("works with different dayWidth values", () => {
    const containerWidth = 800
    const dayWidth = 200

    // scrollLeft=0: center at 400px, day index = floor(400/200) = 2
    expect(calculateCenterDayIndex(0, containerWidth, dayWidth)).toBe(2)

    // scrollLeft=600: center at 1000px, day index = floor(1000/200) = 5
    expect(calculateCenterDayIndex(600, containerWidth, dayWidth)).toBe(5)
  })

  test("works with single day visible (containerWidth = dayWidth)", () => {
    const dayWidth = 200
    const containerWidth = 200

    // scrollLeft=1000: center at 1000 + 100 = 1100px
    // Day index: floor(1100 / 200) = 5
    expect(calculateCenterDayIndex(1000, containerWidth, dayWidth)).toBe(5)
  })

  test("returns 0 when center is in first day", () => {
    // scrollLeft=0, containerWidth=100, dayWidth=100
    // Center position: 0 + 50 = 50px
    // Day index: floor(50 / 100) = 0
    expect(calculateCenterDayIndex(0, 100, 100)).toBe(0)
  })

  test("handles large scroll positions", () => {
    const containerWidth = 700
    const dayWidth = 100

    // scrollLeft=5000: center at 5350px, day index = floor(5350/100) = 53
    expect(calculateCenterDayIndex(5000, containerWidth, dayWidth)).toBe(53)
  })

  test("handles narrow containers", () => {
    // Mobile portrait scenario
    const containerWidth = 375
    const dayWidth = 375

    // scrollLeft=2250: center at 2250 + 187.5 = 2437.5px
    // Day index: floor(2437.5 / 375) = 6
    expect(calculateCenterDayIndex(2250, containerWidth, dayWidth)).toBe(6)
  })

  test("handles wide containers", () => {
    // Desktop scenario with narrow days
    const containerWidth = 1440
    const dayWidth = 288

    // scrollLeft=1000: center at 1000 + 720 = 1720px
    // Day index: floor(1720 / 288) = 5
    expect(calculateCenterDayIndex(1000, containerWidth, dayWidth)).toBe(5)
  })
})

describe("detectEdgeProximity", () => {
  test("returns 'start' when near beginning", () => {
    const maxScroll = 1400
    const threshold = 300

    // Within threshold of start
    expect(detectEdgeProximity(0, maxScroll, threshold)).toBe("start")
    expect(detectEdgeProximity(100, maxScroll, threshold)).toBe("start")
    expect(detectEdgeProximity(200, maxScroll, threshold)).toBe("start")
    expect(detectEdgeProximity(299, maxScroll, threshold)).toBe("start")
  })

  test("returns 'end' when near end", () => {
    const maxScroll = 1400
    const threshold = 300

    // Within threshold of end (maxScroll - threshold = 1100)
    expect(detectEdgeProximity(1400, maxScroll, threshold)).toBe("end")
    expect(detectEdgeProximity(1300, maxScroll, threshold)).toBe("end")
    expect(detectEdgeProximity(1200, maxScroll, threshold)).toBe("end")
    expect(detectEdgeProximity(1101, maxScroll, threshold)).toBe("end")
  })

  test("returns null when in middle (safe zone)", () => {
    const maxScroll = 1400
    const threshold = 300

    // Between threshold and (maxScroll - threshold)
    expect(detectEdgeProximity(300, maxScroll, threshold)).toBeNull()
    expect(detectEdgeProximity(500, maxScroll, threshold)).toBeNull()
    expect(detectEdgeProximity(700, maxScroll, threshold)).toBeNull()
    expect(detectEdgeProximity(1000, maxScroll, threshold)).toBeNull()
    expect(detectEdgeProximity(1100, maxScroll, threshold)).toBeNull()
  })

  test("handles exact threshold boundary", () => {
    const maxScroll = 1400
    const threshold = 300

    // Exactly at threshold
    expect(detectEdgeProximity(299, maxScroll, threshold)).toBe("start")
    expect(detectEdgeProximity(300, maxScroll, threshold)).toBeNull()

    // Exactly at end threshold (1100 = 1400 - 300)
    expect(detectEdgeProximity(1100, maxScroll, threshold)).toBeNull()
    expect(detectEdgeProximity(1101, maxScroll, threshold)).toBe("end")
  })

  test("works with different threshold values", () => {
    const maxScroll = 1400

    // Small threshold (100px)
    expect(detectEdgeProximity(50, maxScroll, 100)).toBe("start")
    expect(detectEdgeProximity(100, maxScroll, 100)).toBeNull()
    expect(detectEdgeProximity(1300, maxScroll, 100)).toBeNull()
    expect(detectEdgeProximity(1350, maxScroll, 100)).toBe("end")

    // Large threshold (500px)
    expect(detectEdgeProximity(400, maxScroll, 500)).toBe("start")
    expect(detectEdgeProximity(500, maxScroll, 500)).toBeNull()
    expect(detectEdgeProximity(900, maxScroll, 500)).toBeNull()
    expect(detectEdgeProximity(1000, maxScroll, 500)).toBe("end")
  })

  test("handles zero scroll position", () => {
    expect(detectEdgeProximity(0, 1400, 300)).toBe("start")
  })

  test("handles maxScroll position", () => {
    expect(detectEdgeProximity(1400, 1400, 300)).toBe("end")
  })

  test("handles very small scroll range", () => {
    // maxScroll smaller than threshold
    const maxScroll = 200
    const threshold = 300

    // Everything should be either start or end
    expect(detectEdgeProximity(0, maxScroll, threshold)).toBe("start")
    expect(detectEdgeProximity(100, maxScroll, threshold)).toBe("start")
    expect(detectEdgeProximity(200, maxScroll, threshold)).toBe("start") // 200 < 300
  })

  test("handles overlapping thresholds", () => {
    // When threshold * 2 > maxScroll, start and end zones overlap
    const maxScroll = 400
    const threshold = 300

    // Start zone: scrollLeft < 300
    expect(detectEdgeProximity(200, maxScroll, threshold)).toBe("start")

    // When zones overlap, the function checks "start" first
    // 150 < 300 so returns "start"
    expect(detectEdgeProximity(150, maxScroll, threshold)).toBe("start")

    // End zone: scrollLeft > (maxScroll - threshold) = 100
    // 350 > 100 and 350 >= 300 (not in start zone), so returns "end"
    expect(detectEdgeProximity(350, maxScroll, threshold)).toBe("end")

    // At exactly maxScroll
    expect(detectEdgeProximity(400, maxScroll, threshold)).toBe("end")
  })

  test("returns consistent results for typical virtual scroll scenario", () => {
    // Typical: 21 days * 100px = 2100px total, containerWidth=700, maxScroll=1400
    const maxScroll = 1400
    const threshold = 300 // 3 days * 100px

    // Start zone: 0-299
    expect(detectEdgeProximity(150, maxScroll, threshold)).toBe("start")

    // Safe zone: 300-1100
    expect(detectEdgeProximity(700, maxScroll, threshold)).toBeNull()

    // End zone: 1101-1400
    expect(detectEdgeProximity(1250, maxScroll, threshold)).toBe("end")
  })

  test("handles zero threshold (never triggers)", () => {
    const maxScroll = 1400
    const threshold = 0

    // With zero threshold, should only trigger at exact boundaries
    expect(detectEdgeProximity(0, maxScroll, threshold)).toBeNull()
    expect(detectEdgeProximity(1400, maxScroll, threshold)).toBeNull()
    expect(detectEdgeProximity(700, maxScroll, threshold)).toBeNull()
  })

  test("handles negative scroll values gracefully", () => {
    // Shouldn't occur in practice, but test behavior
    expect(detectEdgeProximity(-100, 1400, 300)).toBe("start")
  })
})
