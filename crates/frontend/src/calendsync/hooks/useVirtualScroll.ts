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

import { isScrollable } from "@core/calendar/navigation"
import {
  calculateDayWidth,
  calculateHighlightedDay,
  calculateRecenterOffset,
  calculateScrollPosition,
  calculateSnapScrollPosition,
  calculateTotalWidth,
  calculateVisibleDays,
  calculateWindowDates,
  calculateWindowStartDate,
  DEFAULT_VIRTUAL_SCROLL_CONFIG,
  isSameCalendarDay,
  shouldRecenter,
  shouldSnapToDay,
  type VirtualScrollConfig,
} from "@core/calendar/virtualScroll"
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react"

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
  /** Callback to trigger navigation feedback (haptic/audio) */
  onNavigationFeedback?: () => void
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

/** Debounce time for scroll-end detection in ms (used for snap behavior) */
const SCROLL_END_DEBOUNCE_MS = 50

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
 *   onNavigationFeedback: () => navigator.vibrate?.(10),
 * })
 */
export function useVirtualScroll(options: UseVirtualScrollOptions): UseVirtualScrollReturn {
  const {
    initialCenterDate,
    containerWidth,
    enabled = true,
    onHighlightedDayChange,
    onNavigationFeedback,
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
  const scrollEndTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const isRecenteringRef = useRef(false)
  const isSnappingRef = useRef(false)
  const isDraggingRef = useRef(false)
  const prevHighlightedRef = useRef<Date>(initialCenterDate)
  const initialPositionSetRef = useRef(false)
  // Pending scroll adjustments to apply synchronously after DOM update
  const pendingScrollAdjustmentRef = useRef<number | null>(null)
  const pendingScrollToRef = useRef<{ position: number; animated: boolean } | null>(null)

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

      // Trigger feedback
      onNavigationFeedback?.()

      // Notify parent
      onHighlightedDayChange?.(newHighlighted)
    }

    // Check if we need to re-center
    const recenterDirection = shouldRecenter(
      scrollLeft,
      totalWidth,
      containerWidth,
      dayWidth,
      config.recenterThreshold,
    )

    if (recenterDirection) {
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
    onNavigationFeedback,
  ])

  /**
   * Handle scroll end - snap to nearest day when in single-day view.
   */
  const handleScrollEnd = useCallback(() => {
    if (
      !enabled ||
      !scrollContainerRef.current ||
      isRecenteringRef.current ||
      isSnappingRef.current
    )
      return
    if (isDraggingRef.current) return // Don't snap while user is still dragging
    if (!shouldSnapToDay(visibleDays)) return

    const { scrollLeft } = scrollContainerRef.current

    // Calculate snap target
    const { targetDate, scrollPosition } = calculateSnapScrollPosition(
      scrollLeft,
      dayWidth,
      containerWidth,
      windowStartDate,
    )

    // Only snap if not already at target position (with small tolerance)
    const tolerance = 2 // pixels
    if (Math.abs(scrollLeft - scrollPosition) > tolerance) {
      isSnappingRef.current = true

      // Use instant snap for faster response
      scrollContainerRef.current.scrollTo({
        left: scrollPosition,
        behavior: "instant",
      })

      // Update highlighted date
      if (!isSameCalendarDay(targetDate, highlightedDate)) {
        setHighlightedDate(targetDate)
        prevHighlightedRef.current = targetDate
        onNavigationFeedback?.()
        onHighlightedDayChange?.(targetDate)
      }

      // Reset snapping flag after animation completes
      setTimeout(() => {
        isSnappingRef.current = false
      }, 150)
    }
  }, [
    enabled,
    visibleDays,
    dayWidth,
    containerWidth,
    windowStartDate,
    highlightedDate,
    onNavigationFeedback,
    onHighlightedDayChange,
  ])

  /**
   * Debounced scroll handler to avoid excessive updates.
   */
  const debouncedScrollHandler = useCallback(() => {
    if (scrollTimeoutRef.current) {
      clearTimeout(scrollTimeoutRef.current)
    }
    scrollTimeoutRef.current = setTimeout(handleScroll, SCROLL_DEBOUNCE_MS)

    // Also set up scroll-end detection for snap behavior
    if (scrollEndTimeoutRef.current) {
      clearTimeout(scrollEndTimeoutRef.current)
    }
    scrollEndTimeoutRef.current = setTimeout(handleScrollEnd, SCROLL_END_DEBOUNCE_MS)
  }, [handleScroll, handleScrollEnd])

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
        // Target is within window, just scroll
        scrollContainerRef.current.scrollTo({
          left: targetScrollPosition,
          behavior: animated ? "smooth" : "instant",
        })
      }

      // Trigger feedback and notify
      if (!isSameCalendarDay(targetDate, highlightedDate)) {
        onNavigationFeedback?.()
        onHighlightedDayChange?.(targetDate)
      }
    },
    [
      windowStartDate,
      dayWidth,
      containerWidth,
      highlightedDate,
      config.bufferDays,
      onNavigationFeedback,
      onHighlightedDayChange,
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
      if (scrollEndTimeoutRef.current) {
        clearTimeout(scrollEndTimeoutRef.current)
      }
    }
  }, [debouncedScrollHandler, enabled])

  // Track touch/mouse drag state to prevent snapping during active drag
  useEffect(() => {
    const container = scrollContainerRef.current
    if (!container || !enabled) return

    const handleDragStart = () => {
      isDraggingRef.current = true
    }

    const handleDragEnd = () => {
      isDraggingRef.current = false
      // Trigger snap check after drag ends
      if (scrollEndTimeoutRef.current) {
        clearTimeout(scrollEndTimeoutRef.current)
      }
      scrollEndTimeoutRef.current = setTimeout(handleScrollEnd, SCROLL_END_DEBOUNCE_MS)
    }

    // Touch events
    container.addEventListener("touchstart", handleDragStart, { passive: true })
    container.addEventListener("touchend", handleDragEnd, { passive: true })
    container.addEventListener("touchcancel", handleDragEnd, { passive: true })

    // Mouse events (for desktop drag scrolling)
    container.addEventListener("mousedown", handleDragStart, { passive: true })
    window.addEventListener("mouseup", handleDragEnd, { passive: true })

    return () => {
      container.removeEventListener("touchstart", handleDragStart)
      container.removeEventListener("touchend", handleDragEnd)
      container.removeEventListener("touchcancel", handleDragEnd)
      container.removeEventListener("mousedown", handleDragStart)
      window.removeEventListener("mouseup", handleDragEnd)
    }
  }, [enabled, handleScrollEnd])

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
