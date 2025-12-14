import { describe, expect, test } from "bun:test"
import {
  calculateAnimationProgress,
  calculateCurrentScrollPosition,
  calculateScaledDuration,
  calculateScrollDistance,
  createAnimationState,
  easeOutCubic,
  isAnimationComplete,
} from "../scrollAnimation"

describe("easeOutCubic", () => {
  test("returns 0 at start", () => {
    expect(easeOutCubic(0)).toBe(0)
  })

  test("returns 1 at end", () => {
    expect(easeOutCubic(1)).toBe(1)
  })

  test("returns value greater than input (ease-out acceleration)", () => {
    // Ease-out starts fast, so at t=0.5 we should be past 0.5
    expect(easeOutCubic(0.5)).toBeGreaterThan(0.5)
  })

  test("returns value close to 1 near end", () => {
    expect(easeOutCubic(0.9)).toBeGreaterThan(0.95)
  })

  test("is monotonically increasing", () => {
    let prev = 0
    for (let t = 0; t <= 1; t += 0.1) {
      const current = easeOutCubic(t)
      expect(current).toBeGreaterThanOrEqual(prev)
      prev = current
    }
  })
})

describe("createAnimationState", () => {
  test("creates state with correct values", () => {
    const state = createAnimationState(100, 500, 180, 1000)
    expect(state.startPosition).toBe(100)
    expect(state.targetPosition).toBe(500)
    expect(state.duration).toBe(180)
    expect(state.startTime).toBe(1000)
    expect(state.isActive).toBe(true)
  })
})

describe("calculateAnimationProgress", () => {
  test("returns 0 at start", () => {
    expect(calculateAnimationProgress(1000, 1000, 200)).toBe(0)
  })

  test("returns 0.5 at midpoint", () => {
    expect(calculateAnimationProgress(1100, 1000, 200)).toBe(0.5)
  })

  test("returns 1 at end", () => {
    expect(calculateAnimationProgress(1200, 1000, 200)).toBe(1)
  })

  test("caps at 1 when past duration", () => {
    expect(calculateAnimationProgress(1500, 1000, 200)).toBe(1)
  })

  test("handles zero duration", () => {
    expect(calculateAnimationProgress(1000, 1000, 0)).toBe(1)
  })
})

describe("calculateCurrentScrollPosition", () => {
  test("returns start position at animation start", () => {
    const state = createAnimationState(0, 500, 200, 1000)
    const position = calculateCurrentScrollPosition(state, 1000, easeOutCubic)
    expect(position).toBe(0)
  })

  test("returns target position at animation end", () => {
    const state = createAnimationState(0, 500, 200, 1000)
    const position = calculateCurrentScrollPosition(state, 1200, easeOutCubic)
    expect(position).toBe(500)
  })

  test("returns intermediate position during animation", () => {
    const state = createAnimationState(0, 500, 200, 1000)
    const position = calculateCurrentScrollPosition(state, 1100, easeOutCubic)
    expect(position).toBeGreaterThan(0)
    expect(position).toBeLessThan(500)
    // With ease-out, at 50% time we should be past 50% distance
    expect(position).toBeGreaterThan(250)
  })

  test("works with negative scroll direction", () => {
    const state = createAnimationState(500, 0, 200, 1000)
    const position = calculateCurrentScrollPosition(state, 1200, easeOutCubic)
    expect(position).toBe(0)
  })

  test("works with non-zero start position", () => {
    const state = createAnimationState(100, 300, 200, 1000)
    const position = calculateCurrentScrollPosition(state, 1200, easeOutCubic)
    expect(position).toBe(300)
  })
})

describe("isAnimationComplete", () => {
  test("returns false for progress < 1", () => {
    expect(isAnimationComplete(0)).toBe(false)
    expect(isAnimationComplete(0.5)).toBe(false)
    expect(isAnimationComplete(0.99)).toBe(false)
  })

  test("returns true for progress >= 1", () => {
    expect(isAnimationComplete(1)).toBe(true)
    expect(isAnimationComplete(1.1)).toBe(true)
  })
})

describe("calculateScrollDistance", () => {
  test("returns absolute distance", () => {
    expect(calculateScrollDistance(0, 500)).toBe(500)
    expect(calculateScrollDistance(500, 0)).toBe(500)
    expect(calculateScrollDistance(100, 300)).toBe(200)
  })

  test("returns 0 for same position", () => {
    expect(calculateScrollDistance(100, 100)).toBe(0)
  })
})

describe("calculateScaledDuration", () => {
  const baseDuration = 180
  const dayWidth = 200

  test("returns base duration for short distances", () => {
    expect(calculateScaledDuration(100, baseDuration, dayWidth)).toBe(baseDuration)
    expect(calculateScaledDuration(200, baseDuration, dayWidth)).toBe(baseDuration)
  })

  test("increases duration for longer distances", () => {
    const shortDuration = calculateScaledDuration(100, baseDuration, dayWidth)
    const longDuration = calculateScaledDuration(500, baseDuration, dayWidth)
    expect(longDuration).toBeGreaterThan(shortDuration)
  })

  test("caps at 1.5x base duration", () => {
    const result = calculateScaledDuration(2000, baseDuration, dayWidth)
    expect(result).toBeLessThanOrEqual(baseDuration * 1.5)
  })

  test("returns base duration for exactly one day width", () => {
    expect(calculateScaledDuration(dayWidth, baseDuration, dayWidth)).toBe(baseDuration)
  })
})
