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
 * Calculate scroll position to center a target date.
 * Returns null if not scrollable.
 * @param targetDayIndex - The index of the day to center (0-based)
 * @param dayWidth - The width of each day column in pixels
 * @param containerWidth - The visible width of the container
 * @param totalContentWidth - The total width of all content
 * @returns The scroll position in pixels, or null if not scrollable
 */
export function calculateCenteredScrollPosition(
  targetDayIndex: number,
  dayWidth: number,
  containerWidth: number,
  totalContentWidth: number,
): number | null {
  if (totalContentWidth <= containerWidth) return null

  const targetLeft = targetDayIndex * dayWidth
  const centerOffset = (containerWidth - dayWidth) / 2
  const scrollPosition = targetLeft - centerOffset

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
