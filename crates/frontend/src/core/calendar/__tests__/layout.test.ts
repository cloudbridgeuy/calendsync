import { describe, expect, test } from "bun:test"
import {
  calculateAnimationDuration,
  calculateDayPosition,
  calculateDaysFromWheelDelta,
  calculateDayWidth,
  calculateOffsetFromCenter,
  calculateSwipeTransform,
  calculateVisibleDays,
  calculateWheelDragOffset,
  detectWheelDirection,
  getVisibleDateOffsets,
  getWheelNavigationDelta,
  isMobileViewport,
  shouldLoadMoreDays,
  shouldNavigateFromSwipe,
  snapToNearestDay,
} from "../layout"
import { DEFAULT_LAYOUT_CONSTANTS } from "../types"

describe("calculateVisibleDays", () => {
  test("returns 1 for mobile width", () => {
    expect(calculateVisibleDays(500)).toBe(1)
  })

  test("returns 3 for narrow desktop", () => {
    // minDayWidth is 250, so 750 fits 3 days
    expect(calculateVisibleDays(800)).toBe(3)
  })

  test("returns 5 for medium desktop", () => {
    // 1250 fits 5 days at 250 each
    expect(calculateVisibleDays(1300)).toBe(5)
  })

  test("returns 7 for wide desktop", () => {
    // 1750 fits 7 days at 250 each
    expect(calculateVisibleDays(1800)).toBe(7)
  })

  test("caps at 7 days", () => {
    expect(calculateVisibleDays(3000)).toBe(7)
  })

  test("respects custom constants", () => {
    const constants = {
      ...DEFAULT_LAYOUT_CONSTANTS,
      mobileBreakpoint: 1200,
    }
    expect(calculateVisibleDays(1000, constants)).toBe(1)
  })
})

describe("calculateDayWidth", () => {
  test("divides width evenly", () => {
    expect(calculateDayWidth(1000, 5)).toBe(200)
  })

  test("handles single day", () => {
    expect(calculateDayWidth(500, 1)).toBe(500)
  })
})

describe("isMobileViewport", () => {
  test("returns true below breakpoint", () => {
    expect(isMobileViewport(700)).toBe(true)
  })

  test("returns false at breakpoint", () => {
    expect(isMobileViewport(768)).toBe(false)
  })

  test("returns false above breakpoint", () => {
    expect(isMobileViewport(1024)).toBe(false)
  })
})

describe("calculateOffsetFromCenter", () => {
  test("returns 0 for center position", () => {
    // With 5 visible days, center index is 2
    expect(calculateOffsetFromCenter(2, 5)).toBe(0)
  })

  test("returns negative for left of center", () => {
    expect(calculateOffsetFromCenter(0, 5)).toBe(-2)
  })

  test("returns positive for right of center", () => {
    expect(calculateOffsetFromCenter(4, 5)).toBe(2)
  })
})

describe("calculateSwipeTransform", () => {
  test("includes delta and day offset", () => {
    const transform = calculateSwipeTransform(50, 200, 1)
    // base offset is -1 * 200 = -200
    // total is -200 + 50 = -150
    expect(transform).toBe("translateX(-150px)")
  })

  test("handles zero delta", () => {
    const transform = calculateSwipeTransform(0, 200, 0)
    expect(transform).toBe("translateX(0px)")
  })
})

describe("shouldNavigateFromSwipe", () => {
  test("returns navigate for high positive velocity (swipe right = go back)", () => {
    const result = shouldNavigateFromSwipe(10, 0.5)
    expect(result.shouldNavigate).toBe(true)
    expect(result.direction).toBe(-1) // Swipe right = go to previous day
  })

  test("returns navigate for high negative velocity (swipe left = go forward)", () => {
    const result = shouldNavigateFromSwipe(-10, -0.5)
    expect(result.shouldNavigate).toBe(true)
    expect(result.direction).toBe(1) // Swipe left = go to next day
  })

  test("returns navigate for distance threshold", () => {
    const result = shouldNavigateFromSwipe(60, 0.1)
    expect(result.shouldNavigate).toBe(true)
    expect(result.direction).toBe(-1)
  })

  test("returns no navigate for small swipe", () => {
    const result = shouldNavigateFromSwipe(20, 0.1)
    expect(result.shouldNavigate).toBe(false)
    expect(result.direction).toBe(0)
  })
})

describe("getVisibleDateOffsets", () => {
  test("returns centered offsets for odd number", () => {
    const offsets = getVisibleDateOffsets(5)
    expect(offsets).toEqual([-2, -1, 0, 1, 2])
  })

  test("returns centered offsets for 7 days", () => {
    const offsets = getVisibleDateOffsets(7)
    expect(offsets).toEqual([-3, -2, -1, 0, 1, 2, 3])
  })

  test("returns single offset for 1 day", () => {
    const offsets = getVisibleDateOffsets(1)
    expect(offsets).toEqual([0])
  })
})

describe("shouldLoadMoreDays", () => {
  test("indicates load before when near start", () => {
    const result = shouldLoadMoreDays(100, 2000, 200, 3)
    expect(result.loadBefore).toBe(true)
    expect(result.loadAfter).toBe(false)
  })

  test("indicates load after when near end", () => {
    const result = shouldLoadMoreDays(1500, 2000, 200, 3)
    expect(result.loadBefore).toBe(false)
    expect(result.loadAfter).toBe(true)
  })

  test("indicates no load when in middle", () => {
    const result = shouldLoadMoreDays(1000, 2000, 200, 3)
    expect(result.loadBefore).toBe(false)
    expect(result.loadAfter).toBe(false)
  })
})

describe("calculateDayPosition", () => {
  test("calculates position for center day", () => {
    const pos = calculateDayPosition(2, 200, 5)
    expect(pos.width).toBe(200)
    // Center at 2.5 * 200 = 500, minus half width = 400
    expect(pos.left).toBe(400)
  })

  test("calculates position for left day", () => {
    const pos = calculateDayPosition(0, 200, 5)
    // Position 0 is 2 left of center
    // (2.5 + (0 - 2) - 0.5) * 200 = 0
    expect(pos.left).toBe(0)
  })
})

describe("snapToNearestDay", () => {
  test("snaps to nearest day", () => {
    expect(snapToNearestDay(250, 200)).toBe(1)
    expect(snapToNearestDay(150, 200)).toBe(1)
    expect(snapToNearestDay(99, 200)).toBe(0)
  })

  test("handles negative offsets", () => {
    expect(snapToNearestDay(-250, 200)).toBe(-1)
  })
})

describe("calculateAnimationDuration", () => {
  test("returns base duration for short distance", () => {
    const duration = calculateAnimationDuration(0)
    expect(duration).toBe(200)
  })

  test("increases with distance", () => {
    const short = calculateAnimationDuration(100)
    const long = calculateAnimationDuration(300)
    expect(long).toBeGreaterThan(short)
  })

  test("caps at max duration", () => {
    const duration = calculateAnimationDuration(1000)
    expect(duration).toBe(400)
  })

  test("respects custom parameters", () => {
    const duration = calculateAnimationDuration(0, 100, 500)
    expect(duration).toBe(100)
  })
})

// =============================================================================
// Wheel/Trackpad Navigation Functions
// =============================================================================

describe("detectWheelDirection", () => {
  test("returns true (horizontal) when modifier key is pressed", () => {
    expect(detectWheelDirection(0, 10, true)).toBe(true)
    expect(detectWheelDirection(10, 0, true)).toBe(true)
  })

  test("returns null when movement is below threshold", () => {
    expect(detectWheelDirection(3, 2, false)).toBeNull()
    expect(detectWheelDirection(0, 0, false)).toBeNull()
  })

  test("returns true when horizontal movement dominates", () => {
    expect(detectWheelDirection(20, 5, false)).toBe(true)
    expect(detectWheelDirection(-15, 3, false)).toBe(true)
  })

  test("returns false when vertical movement dominates", () => {
    expect(detectWheelDirection(5, 20, false)).toBe(false)
    expect(detectWheelDirection(3, -15, false)).toBe(false)
  })

  test("respects custom threshold", () => {
    expect(detectWheelDirection(8, 2, false, 10)).toBeNull()
    expect(detectWheelDirection(15, 2, false, 10)).toBe(true)
  })
})

describe("getWheelNavigationDelta", () => {
  test("returns deltaY when modifier is pressed", () => {
    expect(getWheelNavigationDelta(10, 50, true)).toBe(50)
    expect(getWheelNavigationDelta(100, -30, true)).toBe(-30)
  })

  test("returns deltaX when no modifier", () => {
    expect(getWheelNavigationDelta(10, 50, false)).toBe(10)
    expect(getWheelNavigationDelta(-25, 100, false)).toBe(-25)
  })
})

describe("calculateWheelDragOffset", () => {
  test("calculates percentage offset correctly", () => {
    // 100px accumulated, 200px day width, 5 visible days
    // -(100/200) * (100/5) = -0.5 * 20 = -10%
    expect(calculateWheelDragOffset(100, 200, 5)).toBe(-10)
  })

  test("returns positive offset for negative delta (dragging right)", () => {
    expect(calculateWheelDragOffset(-100, 200, 5)).toBe(10)
  })

  test("returns 0 for zero delta", () => {
    expect(calculateWheelDragOffset(0, 200, 5)).toBe(0)
  })
})

describe("calculateDaysFromWheelDelta", () => {
  test("rounds to nearest day", () => {
    expect(calculateDaysFromWheelDelta(150, 100)).toBe(2) // 1.5 rounds to 2
    expect(calculateDaysFromWheelDelta(140, 100)).toBe(1) // 1.4 rounds to 1
    expect(calculateDaysFromWheelDelta(50, 100)).toBe(1) // 0.5 rounds to 1
    expect(calculateDaysFromWheelDelta(49, 100)).toBe(0) // 0.49 rounds to 0
  })

  test("handles negative delta (scroll left)", () => {
    // Note: Math.round(-1.5) = -1 in JavaScript (rounds toward +Infinity)
    expect(calculateDaysFromWheelDelta(-160, 100)).toBe(-2) // -1.6 rounds to -2
    expect(calculateDaysFromWheelDelta(-51, 100)).toBe(-1) // -0.51 rounds to -1
  })

  test("returns 0 for small movements", () => {
    expect(calculateDaysFromWheelDelta(10, 100)).toBe(0)
    expect(calculateDaysFromWheelDelta(-10, 100)).toBe(0)
  })
})
