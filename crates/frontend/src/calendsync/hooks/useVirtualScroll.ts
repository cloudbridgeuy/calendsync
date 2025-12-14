/**
 * useVirtualScroll - Hook for infinite horizontal scrolling of calendar days.
 *
 * This hook manages a virtual scroll window that enables infinite scrolling
 * through calendar days using native browser scroll behavior. It handles:
 * - Calculating which days to render based on scroll position
 * - Re-centering the virtual window when approaching edges
 * - Detecting the highlighted day (closest to viewport center)
 * - Triggering feedback when the highlighted day changes
 * - Programmatic scrolling to specific dates
 */

import { focusDayElement, formatDateKey, generateNavigationAnnouncement } from "@core/calendar"
import { isScrollable } from "@core/calendar/navigation"
import {
  calculateDayWidth,
  calculateHighlightedDay,
  calculateRecenterOffset,
  calculateScrollPosition,
  calculateTotalWidth,
  calculateVisibleDays,
  calculateWindowDates,
  calculateWindowStartDate,
  DEFAULT_VIRTUAL_SCROLL_CONFIG,
  isSameCalendarDay,
  shouldRecenter,
  type VirtualScrollConfig,
} from "@core/calendar/virtualScroll"
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react"
import { useAriaAnnouncer } from "./useAriaAnnouncer"
import { useScrollAnimation } from "./useScrollAnimation"

/**
 * Options for the useVirtualScroll hook.
 */
export interface UseVirtualScrollOptions {
  /** Initial center date for the virtual window */
  initialCenterDate: Date
  /** Width of the scroll container viewport in pixels */
  containerWidth: number
  /** Whether scrolling is enabled */
  enabled?: boolean
  /** Callback when highlighted day changes */
  onHighlightedDayChange?: (date: Date) => void
  /** Configuration overrides */
  config?: Partial<VirtualScrollConfig>
}

/**
 * Return value from the useVirtualScroll hook.
 */
export interface UseVirtualScrollReturn {
  /** Ref to attach to the scroll container element */
  scrollContainerRef: React.RefObject<HTMLDivElement | null>
  /** Currently highlighted date (closest to viewport center) */
  highlightedDate: Date
  /** Array of dates to render in the virtual window */
  renderedDates: Date[]
  /** Width of each day column in pixels */
  dayWidth: number
  /** Number of days visible in the viewport */
  visibleDays: number
  /** Scroll to a specific date */
  scrollToDate: (date: Date, animated?: boolean) => void
  /** Jump to today */
  scrollToToday: () => void
}

/** Shift amount when re-centering (number of days) */
const RECENTER_SHIFT_DAYS = 6

/** Debounce time for scroll events in ms (60fps = ~16ms) */
const SCROLL_DEBOUNCE_MS = 16

/**
 * Hook for managing virtual scroll behavior for calendar days.
 *
 * @example
 * const {
 *   scrollContainerRef,
 *   highlightedDate,
 *   renderedDates,
 *   dayWidth,
 *   scrollToDate,
 *   scrollToToday,
 * } = useVirtualScroll({
 *   initialCenterDate: new Date(),
 *   containerWidth: 800,
 *   onHighlightedDayChange: (date) => console.log('New day:', date),
 * })
 */
export function useVirtualScroll(options: UseVirtualScrollOptions): UseVirtualScrollReturn {
  const {
    initialCenterDate,
    containerWidth,
    enabled = true,
    onHighlightedDayChange,
    config: configOverrides,
  } = options

  // Merge config with defaults
  const config = useMemo<VirtualScrollConfig>(
    () => ({
      ...DEFAULT_VIRTUAL_SCROLL_CONFIG,
      ...configOverrides,
    }),
    [configOverrides],
  )

  // Calculate visible days and day width using pure functions from virtualScroll.ts
  const visibleDays = useMemo(() => calculateVisibleDays(containerWidth), [containerWidth])
  const dayWidth = useMemo(
    () => calculateDayWidth(containerWidth, visibleDays),
    [containerWidth, visibleDays],
  )

  // Refs
  const scrollContainerRef = useRef<HTMLDivElement | null>(null)
  const scrollTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const isRecenteringRef = useRef(false)
  const prevHighlightedRef = useRef<Date>(initialCenterDate)
  const initialPositionSetRef = useRef(false)
  // Pending scroll adjustments to apply synchronously after DOM update
  const pendingScrollAdjustmentRef = useRef<number | null>(null)
  const pendingScrollToRef = useRef<{ position: number; animated: boolean } | null>(null)
  // Track the target date for focus after animation completes
  const pendingFocusDateRef = useRef<string | null>(null)

  // ARIA announcer for screen reader notifications
  const { announce } = useAriaAnnouncer()

  // Scroll animation hook for fast, controlled scrolling
  const { animateScrollTo, cancelAnimation, isAnimating } = useScrollAnimation({
    scrollContainerRef,
    dayWidth,
    onComplete: () => {
      // Focus the target day element after animation completes
      if (pendingFocusDateRef.current) {
        focusDayElement(pendingFocusDateRef.current)
        pendingFocusDateRef.current = null
      }
    },
  })

  // State
  const [windowStartDate, setWindowStartDate] = useState<Date>(() =>
    calculateWindowStartDate(initialCenterDate, config.bufferDays),
  )
  const [highlightedDate, setHighlightedDate] = useState<Date>(initialCenterDate)

  // Calculate rendered dates from window start date (not highlighted date)
  // This ensures the window stays stable during scrolling - only shifts when re-centering
  const renderedDates = useMemo(
    () => calculateWindowDates(windowStartDate, config.windowSize),
    [windowStartDate, config.windowSize],
  )

  // Calculate total scrollable width
  const totalWidth = useMemo(
    () => calculateTotalWidth(config.windowSize, dayWidth),
    [config.windowSize, dayWidth],
  )

  /**
   * Handle scroll events - detect highlighted day and re-center if needed.
   */
  const handleScroll = useCallback(() => {
    if (!enabled || !scrollContainerRef.current || isRecenteringRef.current) return

    const { scrollLeft } = scrollContainerRef.current

    // Calculate highlighted day from scroll position
    const newHighlighted = calculateHighlightedDay(
      scrollLeft,
      containerWidth,
      dayWidth,
      windowStartDate,
    )

    // Check if highlighted day changed
    if (!isSameCalendarDay(newHighlighted, prevHighlightedRef.current)) {
      setHighlightedDate(newHighlighted)
      prevHighlightedRef.current = newHighlighted

      // Notify parent
      onHighlightedDayChange?.(newHighlighted)
    }

    // Check if we need to re-center (skip during animation to prevent conflicts)
    const recenterDirection = shouldRecenter(
      scrollLeft,
      totalWidth,
      containerWidth,
      dayWidth,
      config.recenterThreshold,
    )

    if (recenterDirection && !isAnimating()) {
      isRecenteringRef.current = true

      const { newWindowStartDate, scrollAdjustment } = calculateRecenterOffset(
        recenterDirection,
        windowStartDate,
        dayWidth,
        RECENTER_SHIFT_DAYS,
      )

      // Store adjustment to apply after DOM update (not after paint)
      pendingScrollAdjustmentRef.current = scrollAdjustment

      // Triggers re-render with new dates
      setWindowStartDate(newWindowStartDate)
    }
  }, [
    enabled,
    containerWidth,
    dayWidth,
    windowStartDate,
    totalWidth,
    config.recenterThreshold,
    onHighlightedDayChange,
    isAnimating,
  ])

  /**
   * Debounced scroll handler to avoid excessive updates.
   */
  const debouncedScrollHandler = useCallback(() => {
    if (scrollTimeoutRef.current) {
      clearTimeout(scrollTimeoutRef.current)
    }
    scrollTimeoutRef.current = setTimeout(handleScroll, SCROLL_DEBOUNCE_MS)
  }, [handleScroll])

  /**
   * Scroll to a specific date.
   */
  const scrollToDate = useCallback(
    (targetDate: Date, animated = true) => {
      if (!scrollContainerRef.current) return

      // Check if target is within current window
      const targetScrollPosition = calculateScrollPosition(
        targetDate,
        windowStartDate,
        dayWidth,
        containerWidth,
      )

      // If target is far outside current window, re-center window first
      const currentCenter = highlightedDate
      const daysDiff = Math.abs(
        Math.round((targetDate.getTime() - currentCenter.getTime()) / (24 * 60 * 60 * 1000)),
      )

      if (daysDiff > config.bufferDays) {
        // Target is outside buffer, need to shift window
        // Use native scroll for far-away dates (they aren't clickable anyway)
        const newWindowStart = calculateWindowStartDate(targetDate, config.bufferDays)
        const newScrollPosition = calculateScrollPosition(
          targetDate,
          newWindowStart,
          dayWidth,
          containerWidth,
        )

        // Store scroll position to apply after DOM update
        pendingScrollToRef.current = { position: newScrollPosition, animated }

        setWindowStartDate(newWindowStart)
        setHighlightedDate(targetDate)
        prevHighlightedRef.current = targetDate
      } else {
        // Target is within window - use custom animation for fast, reliable scrolling
        if (animated) {
          // Cancel any ongoing animation/momentum
          cancelAnimation()

          // Store target date for focus after animation completes
          pendingFocusDateRef.current = formatDateKey(targetDate)

          // Announce navigation to screen readers
          announce(generateNavigationAnnouncement(targetDate))

          // Start the custom scroll animation
          animateScrollTo(targetScrollPosition)
        } else {
          scrollContainerRef.current.scrollTo({
            left: targetScrollPosition,
            behavior: "instant",
          })
        }
      }

      // Notify parent
      if (!isSameCalendarDay(targetDate, highlightedDate)) {
        onHighlightedDayChange?.(targetDate)
      }
    },
    [
      windowStartDate,
      dayWidth,
      containerWidth,
      highlightedDate,
      config.bufferDays,
      onHighlightedDayChange,
      cancelAnimation,
      animateScrollTo,
      announce,
    ],
  )

  /**
   * Scroll to today's date.
   */
  const scrollToToday = useCallback(() => {
    const today = new Date()
    today.setHours(0, 0, 0, 0)
    scrollToDate(today)
  }, [scrollToDate])

  // Set up scroll event listener
  useEffect(() => {
    const container = scrollContainerRef.current
    if (!container || !enabled) return

    container.addEventListener("scroll", debouncedScrollHandler, { passive: true })

    return () => {
      container.removeEventListener("scroll", debouncedScrollHandler)
      if (scrollTimeoutRef.current) {
        clearTimeout(scrollTimeoutRef.current)
      }
    }
  }, [debouncedScrollHandler, enabled])

  // Initialize scroll position on mount (only once)
  useLayoutEffect(() => {
    if (initialPositionSetRef.current || !scrollContainerRef.current) return
    if (containerWidth <= 0 || dayWidth <= 0) return

    const { scrollWidth, clientWidth } = scrollContainerRef.current

    // Guard: wait until content is scrollable
    if (!isScrollable(scrollWidth, clientWidth)) return

    const initialScrollPosition = calculateScrollPosition(
      highlightedDate,
      windowStartDate,
      dayWidth,
      containerWidth,
    )

    scrollContainerRef.current.scrollLeft = initialScrollPosition
    initialPositionSetRef.current = true
  }, [containerWidth, dayWidth, highlightedDate, windowStartDate]) // Only depend on size changes, not state

  // Apply pending scroll adjustments synchronously after DOM update, before paint.
  // This prevents the 1-frame flash where new dates appear at old scroll position.
  // biome-ignore lint/correctness/useExhaustiveDependencies: windowStartDate triggers the effect when window shifts
  useLayoutEffect(() => {
    if (!scrollContainerRef.current) return

    // Handle re-centering adjustment (relative scroll)
    if (pendingScrollAdjustmentRef.current !== null) {
      scrollContainerRef.current.scrollLeft += pendingScrollAdjustmentRef.current
      pendingScrollAdjustmentRef.current = null
      isRecenteringRef.current = false
    }

    // Handle scrollToDate navigation (absolute scroll)
    if (pendingScrollToRef.current !== null) {
      const { position, animated } = pendingScrollToRef.current
      scrollContainerRef.current.scrollTo({
        left: position,
        behavior: animated ? "smooth" : "instant",
      })
      pendingScrollToRef.current = null
    }
  }, [windowStartDate])

  return {
    scrollContainerRef,
    highlightedDate,
    renderedDates,
    dayWidth,
    visibleDays,
    scrollToDate,
    scrollToToday,
  }
}
