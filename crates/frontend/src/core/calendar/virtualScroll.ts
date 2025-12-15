/**
 * Virtual scroll calculations for infinite horizontal day navigation.
 *
 * This module contains pure functions for managing a virtual scroll window
 * that enables infinite scrolling through calendar days using native browser
 * scroll behavior.
 *
 * Architecture:
 * - Render a fixed window of days (e.g., 21 days: 10 buffer + 1 center + 10 buffer)
 * - When user scrolls near edge, shift the window and adjust scrollLeft instantly
 * - Browser handles all scroll physics (momentum, rubber-banding, etc.)
 */

import { addDays, formatDateKey } from "./dates"

/**
 * Configuration for virtual scrolling behavior.
 */
export interface VirtualScrollConfig {
  /** Total days in virtual window (should be odd for symmetric buffer) */
  windowSize: number
  /** Days of buffer on each side of visible area */
  bufferDays: number
  /** Threshold days from edge to trigger re-center */
  recenterThreshold: number
}

/**
 * Default virtual scroll configuration.
 * 21 days total: 10 buffer + 1 visible center + 10 buffer
 * Re-center when within 3 days of edge.
 */
export const DEFAULT_VIRTUAL_SCROLL_CONFIG: VirtualScrollConfig = {
  windowSize: 21,
  bufferDays: 10,
  recenterThreshold: 3,
}

/**
 * Result of re-center calculation.
 */
export interface RecenterResult {
  /** New start date for the virtual window */
  newWindowStartDate: Date
  /** Scroll adjustment in pixels (add to scrollLeft) */
  scrollAdjustment: number
}

/**
 * Calculate the array of dates for the virtual window centered on a given date.
 *
 * @param centerDate - The date to center the window on
 * @param config - Virtual scroll configuration
 * @returns Array of dates from window start to window end
 *
 * @example
 * const dates = calculateVirtualWindow(new Date("2025-01-15"), config)
 * // Returns 21 dates from Jan 5 to Jan 25 (10 before + center + 10 after)
 */
export function calculateVirtualWindow(
  centerDate: Date,
  config: VirtualScrollConfig = DEFAULT_VIRTUAL_SCROLL_CONFIG,
): Date[] {
  const { windowSize, bufferDays } = config
  const dates: Date[] = []
  const startDate = addDays(centerDate, -bufferDays)

  for (let i = 0; i < windowSize; i++) {
    dates.push(addDays(startDate, i))
  }

  return dates
}

/**
 * Calculate the scrollLeft value needed to center a group of visible days in the viewport.
 *
 * This function positions the scroll container so that the target date appears in the center
 * of the visible day columns. For odd numbers (1, 3, 5, 7), the target is exactly centered.
 * For even numbers (2), the target appears slightly right of center.
 *
 * Special case: When dayWidth doesn't fill the viewport (e.g., 75% width for 500-749px range),
 * the day is centered in the viewport with partial buffer columns visible on both sides.
 *
 * @param targetDate - The date to center among visible days
 * @param windowStartDate - The first date in the virtual window
 * @param dayWidth - Width of each day column in pixels
 * @param containerWidth - Width of the scroll container viewport
 * @param visibleDays - Number of days that should be visible in the viewport
 * @returns scrollLeft value in pixels
 *
 * @example
 * // Window starts Jan 1, target is Jan 10, 3 visible days, 300px each
 * const scrollLeft = calculateScrollPosition(targetDate, windowStart, 300, 900, 3)
 * // Returns 2400 (days 8, 9, 10 fill viewport, day 10 is right-of-center for 3 days)
 */
export function calculateScrollPosition(
  targetDate: Date,
  windowStartDate: Date,
  dayWidth: number,
  containerWidth: number,
  visibleDays: number,
): number {
  const dayIndex = Math.round(
    (targetDate.getTime() - windowStartDate.getTime()) / (24 * 60 * 60 * 1000),
  )

  // Calculate how many days appear before the centered day
  const daysBeforeCenter = Math.floor(visibleDays / 2)

  // First visible day index
  const firstVisibleDayIndex = dayIndex - daysBeforeCenter

  // Calculate expected total width of visible days
  const expectedVisibleWidth = dayWidth * visibleDays

  // If visible days don't fill the viewport (e.g., 75% width special case),
  // center the day(s) in the viewport
  if (expectedVisibleWidth < containerWidth) {
    const centerOffset = (containerWidth - expectedVisibleWidth) / 2
    return Math.max(0, firstVisibleDayIndex * dayWidth - centerOffset)
  }

  // Normal case: align first visible day with left edge
  return Math.max(0, firstVisibleDayIndex * dayWidth)
}

/**
 * Calculate which date is closest to the center of the viewport.
 *
 * @param scrollLeft - Current scroll position in pixels
 * @param containerWidth - Width of the scroll container viewport
 * @param dayWidth - Width of each day column in pixels
 * @param windowStartDate - The first date in the virtual window
 * @returns The date closest to the viewport center
 *
 * @example
 * const highlighted = calculateHighlightedDay(650, 700, 100, windowStart)
 * // Returns the date at index 10 (center of viewport)
 */
export function calculateHighlightedDay(
  scrollLeft: number,
  containerWidth: number,
  dayWidth: number,
  windowStartDate: Date,
): Date {
  const centerScrollPosition = scrollLeft + containerWidth / 2
  const dayIndex = Math.floor(centerScrollPosition / dayWidth)
  return addDays(windowStartDate, dayIndex)
}

/**
 * Determine if the scroll position is near enough to an edge to require re-centering.
 *
 * @param scrollLeft - Current scroll position in pixels
 * @param totalWidth - Total scrollable width (windowSize * dayWidth)
 * @param containerWidth - Width of the scroll container viewport
 * @param dayWidth - Width of each day column in pixels
 * @param threshold - Number of days from edge to trigger re-center
 * @returns "start" if near start, "end" if near end, null if safe
 *
 * @example
 * const edge = shouldRecenter(50, 2100, 700, 100, 3)
 * // Returns "start" because scrollLeft is within 3 days (300px) of start
 */
export function shouldRecenter(
  scrollLeft: number,
  totalWidth: number,
  containerWidth: number,
  dayWidth: number,
  threshold: number,
): "start" | "end" | null {
  const thresholdPx = threshold * dayWidth
  const maxScroll = totalWidth - containerWidth

  if (scrollLeft < thresholdPx) {
    return "start"
  }

  if (scrollLeft > maxScroll - thresholdPx) {
    return "end"
  }

  return null
}

/**
 * Calculate the new window position and scroll adjustment for seamless re-centering.
 *
 * When the user scrolls near the edge of the virtual window, we need to:
 * 1. Shift the window (change which dates are rendered)
 * 2. Adjust scrollLeft to maintain visual continuity
 *
 * @param direction - Which edge triggered re-center ("start" or "end")
 * @param currentWindowStartDate - Current first date in the virtual window
 * @param dayWidth - Width of each day column in pixels
 * @param shiftDays - Number of days to shift the window
 * @returns New window start date and scroll adjustment
 *
 * @example
 * // User scrolled near end, shift window forward by 6 days
 * const result = calculateRecenterOffset("end", windowStart, 100, 6)
 * // result.newWindowStartDate is 6 days later
 * // result.scrollAdjustment is -600 (scroll back to stay in place visually)
 */
export function calculateRecenterOffset(
  direction: "start" | "end",
  currentWindowStartDate: Date,
  dayWidth: number,
  shiftDays: number,
): RecenterResult {
  if (direction === "end") {
    return {
      newWindowStartDate: addDays(currentWindowStartDate, shiftDays),
      scrollAdjustment: -(shiftDays * dayWidth),
    }
  }
  return {
    newWindowStartDate: addDays(currentWindowStartDate, -shiftDays),
    scrollAdjustment: shiftDays * dayWidth,
  }
}

/**
 * Check if two dates represent the same calendar day.
 *
 * @param a - First date
 * @param b - Second date
 * @returns true if both dates are the same calendar day
 *
 * @example
 * isSameCalendarDay(new Date("2025-01-15T10:00"), new Date("2025-01-15T22:00"))
 * // Returns true
 */
export function isSameCalendarDay(a: Date, b: Date): boolean {
  return formatDateKey(a) === formatDateKey(b)
}

/**
 * Calculate the window start date from a center date.
 *
 * @param centerDate - The date at the center of the window
 * @param bufferDays - Number of buffer days before center
 * @returns The start date of the window
 */
export function calculateWindowStartDate(centerDate: Date, bufferDays: number): Date {
  return addDays(centerDate, -bufferDays)
}

/**
 * Calculate total scrollable width.
 *
 * @param windowSize - Number of days in the window
 * @param dayWidth - Width of each day column in pixels
 * @returns Total width in pixels
 */
export function calculateTotalWidth(windowSize: number, dayWidth: number): number {
  return windowSize * dayWidth
}

/**
 * Calculate the day index from a date within the window.
 *
 * @param date - The date to find the index for
 * @param windowStartDate - The first date in the window
 * @returns Index of the day (0-based)
 */
export function calculateDayIndex(date: Date, windowStartDate: Date): number {
  return Math.round((date.getTime() - windowStartDate.getTime()) / (24 * 60 * 60 * 1000))
}

/**
 * Calculate the number of visible days based on container width.
 *
 * Breakpoints:
 * - < 500px: 1 day (full width)
 * - 500px - 749px: 1 day (75% width - special case for better visual appearance)
 * - 750px - 1249px: 3 days (third width each)
 * - 1250px - 1749px: 5 days (fifth width each)
 * - >= 1750px: 7 days (seventh width each)
 *
 * @param containerWidth - Width of the scroll container viewport in pixels
 * @returns Number of days that should be visible
 *
 * @example
 * calculateVisibleDays(400) // Returns 1 (single day, full width)
 * calculateVisibleDays(600) // Returns 1 (single day, 75% width)
 * calculateVisibleDays(900) // Returns 3 (three days)
 * calculateVisibleDays(1500) // Returns 5 (five days)
 * calculateVisibleDays(1920) // Returns 7 (seven days)
 */
export function calculateVisibleDays(containerWidth: number): number {
  if (containerWidth <= 0) return 1
  if (containerWidth < 500) return 1
  if (containerWidth < 750) return 1 // Special case: 1 day at 75% width
  if (containerWidth < 1250) return 3
  if (containerWidth < 1750) return 5
  return 7
}

/**
 * Calculate the width of each day column in pixels.
 *
 * Divides the container width evenly among the visible days, with a special
 * case for the 500-749px range where the day width is 75% of the container
 * (for better visual appearance with partial buffer columns visible).
 *
 * Returns a fallback value if inputs are invalid.
 *
 * @param containerWidth - Width of the scroll container viewport in pixels
 * @param visibleDays - Number of days that should be visible
 * @returns Width of each day column in pixels
 *
 * @example
 * calculateDayWidth(700, 7) // Returns 100 (100px per day)
 * calculateDayWidth(375, 1) // Returns 375 (full width for single day)
 * calculateDayWidth(600, 1) // Returns 450 (75% width for 500-749px range)
 * calculateDayWidth(0, 5) // Returns 100 (fallback for invalid input)
 */
export function calculateDayWidth(containerWidth: number, visibleDays: number): number {
  if (containerWidth <= 0 || visibleDays <= 0) return 100

  // Special case: 500-749px viewport with 1 visible day uses 75% width
  if (containerWidth >= 500 && containerWidth < 750 && visibleDays === 1) {
    return containerWidth * 0.75
  }

  return containerWidth / visibleDays
}

/**
 * Calculate an array of consecutive dates starting from a given date.
 *
 * Unlike `calculateVirtualWindow` which centers on a date, this function
 * generates dates starting directly from the provided start date. This is
 * useful for maintaining a stable window position during scrolling.
 *
 * @param startDate - The first date in the window
 * @param windowSize - Number of days to generate
 * @returns Array of consecutive dates starting from startDate
 *
 * @example
 * const dates = calculateWindowDates(new Date("2025-01-05"), 21)
 * // Returns 21 dates from Jan 5 to Jan 25
 */
export function calculateWindowDates(startDate: Date, windowSize: number): Date[] {
  if (windowSize <= 0) return []
  const dates: Date[] = []
  for (let i = 0; i < windowSize; i++) {
    dates.push(addDays(startDate, i))
  }
  return dates
}
