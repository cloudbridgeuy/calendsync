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
    expect(calculateCenteredScrollPosition(5, 100, 700, 700, 7)).toBeNull()

    // Total content width less than container width
    expect(calculateCenteredScrollPosition(5, 100, 700, 500, 7)).toBeNull()
  })

  test("centers group of 2 days", () => {
    // targetDayIndex=10, dayWidth=300, container=600, total=6300, visible=2
    // floor(2/2) = 1 day before, first visible = 9
    const result = calculateCenteredScrollPosition(10, 300, 600, 6300, 2)
    expect(result).toBe(2700) // 9 * 300
  })

  test("centers group of 3 days", () => {
    // targetDayIndex=10, dayWidth=300, container=900, total=6300, visible=3
    // floor(3/2) = 1 day before, first visible = 9
    const result = calculateCenteredScrollPosition(10, 300, 900, 6300, 3)
    expect(result).toBe(2700) // 9 * 300
  })

  test("centers group of 5 days", () => {
    // targetDayIndex=10, dayWidth=250, container=1250, total=5250, visible=5
    // floor(5/2) = 2 days before, first visible = 8
    const result = calculateCenteredScrollPosition(10, 250, 1250, 5250, 5)
    expect(result).toBe(2000) // 8 * 250
  })

  test("centers group of 7 days", () => {
    // targetDayIndex=10, dayWidth=250, container=1750, total=5250, visible=7
    // floor(7/2) = 3 days before, first visible = 7
    const result = calculateCenteredScrollPosition(10, 250, 1750, 5250, 7)
    expect(result).toBe(1750) // 7 * 250
  })

  test("clamps to 0 when centering would scroll before start", () => {
    // Day 1 with 3 visible days
    // floor(3/2) = 1 day before, first visible = 0
    const result = calculateCenteredScrollPosition(1, 300, 900, 6300, 3)
    expect(result).toBe(0) // 0 * 300 = 0
  })

  test("clamps to maxScroll when centering would scroll past end", () => {
    // Day 18 in 21-day window, 3 visible days
    // floor(3/2) = 1, first visible = 17
    // Scroll = 17 * 300 = 5100
    // maxScroll = 6300 - 900 = 5400
    const result = calculateCenteredScrollPosition(18, 300, 900, 6300, 3)
    expect(result).toBe(5100) // Within bounds

    // Day 20, would give scroll = 19 * 300 = 5700 > 5400
    const result2 = calculateCenteredScrollPosition(20, 300, 900, 6300, 3)
    expect(result2).toBe(5400) // Clamped to maxScroll
  })

  test("handles day 0 (first day)", () => {
    // Day 0 with 3 visible days
    // floor(3/2) = 1 day before, first visible = -1 -> clamped to 0
    const result = calculateCenteredScrollPosition(0, 300, 900, 6300, 3)
    expect(result).toBe(0)
  })

  test("works with single day visible (visibleDays=1)", () => {
    // Day 5 with visibleDays=1
    // floor(1/2) = 0 days before, first visible = 5
    const result = calculateCenteredScrollPosition(5, 400, 400, 8400, 1)
    expect(result).toBe(2000) // 5 * 400
  })

  test("ensures returned position is within valid range [0, maxScroll]", () => {
    const dayWidth = 300
    const containerWidth = 900
    const totalWidth = 6300
    const visibleDays = 3
    const maxScroll = totalWidth - containerWidth // 5400

    // Test various day indexes
    for (let dayIndex = 0; dayIndex <= 20; dayIndex++) {
      const result = calculateCenteredScrollPosition(
        dayIndex,
        dayWidth,
        containerWidth,
        totalWidth,
        visibleDays,
      )
      expect(result).not.toBeNull()
      expect(result).toBeGreaterThanOrEqual(0)
      expect(result).toBeLessThanOrEqual(maxScroll)
    }
  })

  test("centers day when dayWidth is 75% of container (500-749px special case)", () => {
    const containerWidth = 600
    const dayWidth = 450 // 75% of 600
    const totalWidth = 9450 // 21 days * 450
    const visibleDays = 1
    const targetDayIndex = 10

    const result = calculateCenteredScrollPosition(
      targetDayIndex,
      dayWidth,
      containerWidth,
      totalWidth,
      visibleDays,
    )

    // centerOffset = (600 - 450) / 2 = 75
    // scrollPosition = 10 * 450 - 75 = 4425
    expect(result).toBe(4425)
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
