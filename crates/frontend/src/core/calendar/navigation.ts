// navigation.ts - Pure functions for scroll navigation

/**
 * Check if an element has scrollable content.
 * @param scrollWidth - The total width of the scrollable content
 * @param clientWidth - The visible width of the container
 * @returns true if scrollWidth > clientWidth
 */
export function isScrollable(scrollWidth: number, clientWidth: number): boolean {
  return scrollWidth > clientWidth
}

/**
 * Calculate scroll position to center a group of visible days.
 * Returns null if not scrollable.
 *
 * This function calculates the scroll position needed to show a group of visible days
 * with the target day in the center position. For odd numbers (1, 3, 5, 7), the target
 * is exactly centered. For even numbers (2), the target appears slightly right of center.
 *
 * Special case: When dayWidth doesn't fill the viewport (e.g., 75% width for 500-749px range),
 * the day is centered in the viewport with partial buffer columns visible on both sides.
 *
 * @param targetDayIndex - The index of the day to center (0-based)
 * @param dayWidth - The width of each day column in pixels
 * @param containerWidth - The visible width of the container
 * @param totalContentWidth - The total width of all content
 * @param visibleDays - Number of days that should be visible in the viewport
 * @returns The scroll position in pixels, or null if not scrollable
 */
export function calculateCenteredScrollPosition(
  targetDayIndex: number,
  dayWidth: number,
  containerWidth: number,
  totalContentWidth: number,
  visibleDays: number,
): number | null {
  if (totalContentWidth <= containerWidth) return null

  // Calculate how many days appear before the centered day
  const daysBeforeCenter = Math.floor(visibleDays / 2)

  // First visible day index
  const firstVisibleDayIndex = targetDayIndex - daysBeforeCenter

  // Calculate expected total width of visible days
  const expectedVisibleWidth = dayWidth * visibleDays

  // Calculate scroll position
  let scrollPosition: number

  // If visible days don't fill the viewport (e.g., 75% width special case),
  // center the day(s) in the viewport
  if (expectedVisibleWidth < containerWidth) {
    const centerOffset = (containerWidth - expectedVisibleWidth) / 2
    scrollPosition = firstVisibleDayIndex * dayWidth - centerOffset
  } else {
    // Normal case: align first visible day with left edge
    scrollPosition = firstVisibleDayIndex * dayWidth
  }

  // Clamp to valid scroll range
  const maxScroll = totalContentWidth - containerWidth
  return Math.max(0, Math.min(scrollPosition, maxScroll))
}

/**
 * Calculate which day index is at the center of the viewport.
 * @param scrollLeft - The current scroll position
 * @param containerWidth - The visible width of the container
 * @param dayWidth - The width of each day column in pixels
 * @returns The index of the day at the center
 */
export function calculateCenterDayIndex(
  scrollLeft: number,
  containerWidth: number,
  dayWidth: number,
): number {
  const centerPosition = scrollLeft + containerWidth / 2
  return Math.floor(centerPosition / dayWidth)
}

/**
 * Determine if we're near an edge and need to re-center.
 * @param scrollLeft - The current scroll position
 * @param maxScroll - The maximum scroll position
 * @param thresholdPx - The distance in pixels from edge to trigger
 * @returns "start" if near start, "end" if near end, null otherwise
 */
export function detectEdgeProximity(
  scrollLeft: number,
  maxScroll: number,
  thresholdPx: number,
): "start" | "end" | null {
  if (scrollLeft < thresholdPx) return "start"
  if (scrollLeft > maxScroll - thresholdPx) return "end"
  return null
}
