/**
 * Pure layout calculation functions.
 * These functions compute positions and sizes without touching the DOM.
 */

import type { LayoutConstants } from "./types"
import { DEFAULT_LAYOUT_CONSTANTS } from "./types"

/**
 * Calculate how many days should be visible based on container width.
 */
export function calculateVisibleDays(
  containerWidth: number,
  constants: LayoutConstants = DEFAULT_LAYOUT_CONSTANTS,
): number {
  const { minDayWidth, mobileBreakpoint } = constants

  // Mobile: always show 1 day
  if (containerWidth < mobileBreakpoint) {
    return 1
  }

  // Desktop: calculate based on min width per day
  const maxDays = Math.floor(containerWidth / minDayWidth)

  // Return odd number so there's a clear center day (3, 5, or 7)
  if (maxDays >= 7) return 7
  if (maxDays >= 5) return 5
  if (maxDays >= 3) return 3
  return 1
}

/**
 * Calculate the width of each day column.
 */
export function calculateDayWidth(containerWidth: number, visibleDays: number): number {
  return containerWidth / visibleDays
}

/**
 * Check if the viewport is considered mobile.
 */
export function isMobileViewport(
  containerWidth: number,
  constants: LayoutConstants = DEFAULT_LAYOUT_CONSTANTS,
): boolean {
  return containerWidth < constants.mobileBreakpoint
}

/**
 * Calculate the offset index from center for a given position.
 * Used for keyboard navigation and swipe handling.
 */
export function calculateOffsetFromCenter(position: number, visibleDays: number): number {
  const centerIndex = Math.floor(visibleDays / 2)
  return position - centerIndex
}

/**
 * Calculate the translation for swipe gestures.
 * Returns a CSS transform value.
 */
export function calculateSwipeTransform(
  deltaX: number,
  dayWidth: number,
  dayOffset: number,
): string {
  const baseOffset = -dayOffset * dayWidth
  return `translateX(${baseOffset + deltaX}px)`
}

/**
 * Determine if a swipe should trigger navigation.
 */
export function shouldNavigateFromSwipe(
  deltaX: number,
  velocity: number,
  constants: LayoutConstants = DEFAULT_LAYOUT_CONSTANTS,
): { shouldNavigate: boolean; direction: -1 | 0 | 1 } {
  const { swipeThreshold, velocityThreshold } = constants

  // Check velocity first (fast swipe)
  if (Math.abs(velocity) > velocityThreshold) {
    return {
      shouldNavigate: true,
      direction: velocity > 0 ? -1 : 1,
    }
  }

  // Check distance threshold
  if (Math.abs(deltaX) > swipeThreshold) {
    return {
      shouldNavigate: true,
      direction: deltaX > 0 ? -1 : 1,
    }
  }

  return { shouldNavigate: false, direction: 0 }
}

/**
 * Calculate the visible date range given center date and visible days.
 * Returns array of date offsets from center.
 */
export function getVisibleDateOffsets(visibleDays: number): number[] {
  const offsets: number[] = []
  const halfDays = Math.floor(visibleDays / 2)

  for (let i = -halfDays; i <= halfDays; i++) {
    // Normalize -0 to 0
    offsets.push(i === 0 ? 0 : i)
  }

  // Handle even numbers of visible days
  if (offsets.length < visibleDays) {
    offsets.push(halfDays + 1)
  }

  return offsets.slice(0, visibleDays)
}

/**
 * Calculate scroll position for infinite scroll effect.
 * Used to determine when to load more data.
 */
export function shouldLoadMoreDays(
  scrollPosition: number,
  totalWidth: number,
  dayWidth: number,
  bufferDays: number,
): { loadBefore: boolean; loadAfter: boolean } {
  const bufferWidth = bufferDays * dayWidth

  return {
    loadBefore: scrollPosition < bufferWidth,
    loadAfter: scrollPosition > totalWidth - bufferWidth,
  }
}

/**
 * Calculate the position of a day within the viewport.
 * Returns { left, width } for positioning.
 */
export function calculateDayPosition(
  dayIndex: number,
  dayWidth: number,
  visibleDays: number,
): { left: number; width: number } {
  const centerIndex = Math.floor(visibleDays / 2)
  const offsetFromCenter = dayIndex - centerIndex
  const left = (visibleDays / 2 + offsetFromCenter - 0.5) * dayWidth

  return { left, width: dayWidth }
}

/**
 * Snap to the nearest day after dragging.
 * Returns the offset to apply.
 */
export function snapToNearestDay(currentOffset: number, dayWidth: number): number {
  return Math.round(currentOffset / dayWidth)
}

/**
 * Calculate animation duration based on distance.
 * Longer distances get longer animations.
 */
export function calculateAnimationDuration(
  distance: number,
  baseDuration: number = 200,
  maxDuration: number = 400,
): number {
  const calculated = baseDuration + Math.abs(distance) * 0.5
  return Math.min(calculated, maxDuration)
}

// =============================================================================
// Wheel/Trackpad Navigation Functions
// =============================================================================

/**
 * Determine if a wheel event indicates horizontal scrolling intent.
 * Returns null if movement is too small to determine direction.
 *
 * Direction lock: once determined, the gesture should stick to horizontal or vertical.
 */
export function detectWheelDirection(
  deltaX: number,
  deltaY: number,
  hasModifier: boolean,
  threshold: number = 5,
): boolean | null {
  // Modifier key always means horizontal (for day navigation)
  if (hasModifier) return true

  // Need significant movement to determine direction
  if (Math.abs(deltaX) <= threshold && Math.abs(deltaY) <= threshold) {
    return null
  }

  return Math.abs(deltaX) > Math.abs(deltaY)
}

/**
 * Calculate the effective delta for wheel navigation.
 * Uses deltaY for modifier+scroll, deltaX for regular horizontal scroll.
 */
export function getWheelNavigationDelta(
  deltaX: number,
  deltaY: number,
  hasModifier: boolean,
): number {
  return hasModifier ? deltaY : deltaX
}

/**
 * Calculate drag offset percentage from accumulated wheel delta.
 * Used for visual feedback during trackpad gestures.
 */
export function calculateWheelDragOffset(
  accumulatedDelta: number,
  dayWidth: number,
  visibleDays: number,
): number {
  const result = -(accumulatedDelta / dayWidth) * (100 / visibleDays)
  // Normalize -0 to 0 for consistency
  return result === 0 ? 0 : result
}

/**
 * Calculate number of days to navigate based on accumulated scroll.
 * Rounds to nearest whole day for snap behavior.
 */
export function calculateDaysFromWheelDelta(accumulatedDelta: number, dayWidth: number): number {
  const result = Math.round(accumulatedDelta / dayWidth)
  // Normalize -0 to 0 for consistency
  return result === 0 ? 0 : result
}
