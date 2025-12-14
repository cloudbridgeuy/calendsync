/**
 * useScrollAnimation - Custom hook for controlled scroll animations.
 *
 * Provides:
 * - Fast, controlled scroll animation using requestAnimationFrame
 * - Momentum scroll cancellation
 * - Animation interruption for new scroll requests
 */

import {
  calculateCurrentScrollPosition,
  calculateScaledDuration,
  createAnimationState,
  DEFAULT_SCROLL_ANIMATION_CONFIG,
  isAnimationComplete,
  type ScrollAnimationConfig,
  type ScrollAnimationState,
} from "@core/calendar"
import { useCallback, useRef } from "react"

export interface UseScrollAnimationOptions {
  /** Ref to the scroll container element */
  scrollContainerRef: React.RefObject<HTMLDivElement | null>
  /** Day width for duration scaling */
  dayWidth: number
  /** Animation configuration */
  config?: Partial<ScrollAnimationConfig>
  /** Callback when animation completes successfully */
  onComplete?: () => void
}

export interface UseScrollAnimationReturn {
  /** Start animated scroll to a position */
  animateScrollTo: (targetPosition: number) => void
  /** Cancel any ongoing animation */
  cancelAnimation: () => void
  /** Check if animation is currently running */
  isAnimating: () => boolean
}

/**
 * Hook for managing scroll animations with cancellation support.
 */
export function useScrollAnimation(options: UseScrollAnimationOptions): UseScrollAnimationReturn {
  const { scrollContainerRef, dayWidth, config: configOverride, onComplete } = options

  // Merge config with defaults
  const config: ScrollAnimationConfig = {
    ...DEFAULT_SCROLL_ANIMATION_CONFIG,
    ...configOverride,
  }

  // Animation state refs (mutable, no re-renders)
  const animationStateRef = useRef<ScrollAnimationState | null>(null)
  const rafIdRef = useRef<number | null>(null)

  /**
   * Cancel any ongoing animation and momentum scroll.
   */
  const cancelAnimation = useCallback(() => {
    // Cancel RAF
    if (rafIdRef.current !== null) {
      cancelAnimationFrame(rafIdRef.current)
      rafIdRef.current = null
    }

    // Mark animation as inactive
    if (animationStateRef.current) {
      animationStateRef.current.isActive = false
    }

    // Stop momentum scroll by setting scroll position to current
    // This is the key trick - setting scrollLeft to itself cancels momentum
    const container = scrollContainerRef.current
    if (container) {
      // biome-ignore lint/correctness/noSelfAssign: Intentional - assigning scrollLeft to itself cancels browser momentum scroll
      container.scrollLeft = container.scrollLeft
    }
  }, [scrollContainerRef])

  /**
   * Animation frame callback.
   */
  const tick = useCallback(
    (currentTime: number) => {
      const state = animationStateRef.current
      const container = scrollContainerRef.current

      if (!state || !state.isActive || !container) {
        return
      }

      // Calculate current position
      const progress = (currentTime - state.startTime) / state.duration
      const currentPosition = calculateCurrentScrollPosition(state, currentTime, config.easing)

      // Apply scroll position (instant, no smooth)
      container.scrollTo({
        left: currentPosition,
        behavior: "instant",
      })

      // Check if complete
      if (isAnimationComplete(progress)) {
        animationStateRef.current = null
        rafIdRef.current = null
        onComplete?.()
        return
      }

      // Schedule next frame
      rafIdRef.current = requestAnimationFrame(tick)
    },
    [scrollContainerRef, config.easing, onComplete],
  )

  /**
   * Start animated scroll to target position.
   */
  const animateScrollTo = useCallback(
    (targetPosition: number) => {
      const container = scrollContainerRef.current
      if (!container) return

      // Cancel any existing animation first
      cancelAnimation()

      const startPosition = container.scrollLeft
      const distance = Math.abs(targetPosition - startPosition)

      // Skip animation if already at target
      if (distance < 1) {
        onComplete?.()
        return
      }

      // Calculate duration based on distance
      const duration = calculateScaledDuration(distance, config.duration, dayWidth)

      // Create animation state
      animationStateRef.current = createAnimationState(
        startPosition,
        targetPosition,
        duration,
        performance.now(),
      )

      // Start animation loop
      rafIdRef.current = requestAnimationFrame(tick)
    },
    [scrollContainerRef, cancelAnimation, config.duration, dayWidth, onComplete, tick],
  )

  /**
   * Check if animation is currently active.
   */
  const isAnimating = useCallback(() => {
    return animationStateRef.current?.isActive ?? false
  }, [])

  return {
    animateScrollTo,
    cancelAnimation,
    isAnimating,
  }
}
