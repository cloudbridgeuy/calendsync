/**
 * Pure scroll animation calculations.
 * No side effects - these are part of the Functional Core.
 */

/** Easing function type */
export type EasingFunction = (t: number) => number

/**
 * Cubic ease-out function - fast start, slow end.
 * Feels natural for scroll animations.
 *
 * @param t - Progress from 0 to 1
 * @returns Eased progress from 0 to 1
 */
export function easeOutCubic(t: number): number {
  return 1 - (1 - t) ** 3
}

/** Configuration for scroll animation */
export interface ScrollAnimationConfig {
  /** Animation duration in milliseconds */
  duration: number
  /** Easing function to use */
  easing: EasingFunction
}

/** Default scroll animation configuration */
export const DEFAULT_SCROLL_ANIMATION_CONFIG: ScrollAnimationConfig = {
  duration: 180,
  easing: easeOutCubic,
}

/** State of an ongoing scroll animation */
export interface ScrollAnimationState {
  /** Starting scroll position in pixels */
  startPosition: number
  /** Target scroll position in pixels */
  targetPosition: number
  /** Animation start timestamp */
  startTime: number
  /** Animation duration in milliseconds */
  duration: number
  /** Whether animation is active */
  isActive: boolean
}

/**
 * Create initial animation state for a scroll.
 */
export function createAnimationState(
  startPosition: number,
  targetPosition: number,
  duration: number,
  startTime: number,
): ScrollAnimationState {
  return {
    startPosition,
    targetPosition,
    startTime,
    duration,
    isActive: true,
  }
}

/**
 * Calculate animation progress (0 to 1) from elapsed time.
 * Returns 1 if animation is complete or duration is zero.
 */
export function calculateAnimationProgress(
  currentTime: number,
  startTime: number,
  duration: number,
): number {
  if (duration <= 0) return 1
  const elapsed = currentTime - startTime
  return Math.min(1, elapsed / duration)
}

/**
 * Calculate current scroll position based on animation state and time.
 *
 * @param state - Current animation state
 * @param currentTime - Current timestamp from performance.now()
 * @param easing - Easing function to apply
 * @returns Current scroll position in pixels
 */
export function calculateCurrentScrollPosition(
  state: ScrollAnimationState,
  currentTime: number,
  easing: EasingFunction,
): number {
  const progress = calculateAnimationProgress(currentTime, state.startTime, state.duration)
  const easedProgress = easing(progress)
  const distance = state.targetPosition - state.startPosition
  return state.startPosition + distance * easedProgress
}

/**
 * Check if animation is complete based on progress.
 */
export function isAnimationComplete(progress: number): boolean {
  return progress >= 1
}

/**
 * Calculate distance between current and target scroll positions.
 */
export function calculateScrollDistance(currentPosition: number, targetPosition: number): number {
  return Math.abs(targetPosition - currentPosition)
}

/**
 * Calculate scaled animation duration based on distance.
 * Shorter distances get faster animations.
 *
 * @param distance - Distance to scroll in pixels
 * @param baseDuration - Base animation duration
 * @param dayWidth - Width of a day column (for reference)
 * @returns Scaled duration in milliseconds
 */
export function calculateScaledDuration(
  distance: number,
  baseDuration: number,
  dayWidth: number,
): number {
  // Scale: within 1 day = baseDuration, beyond = slightly longer
  const dayRatio = distance / dayWidth
  if (dayRatio <= 1) {
    return baseDuration
  }
  // Max 1.5x duration for multi-day scrolls (still fast)
  return Math.min(baseDuration * 1.5, baseDuration + dayRatio * 10)
}
