/**
 * Calendar state management for navigation and visual feedback.
 *
 * @deprecated PARTIALLY DEPRECATED: The entry caching in this hook should migrate
 * to useOfflineCalendar for offline-first capabilities. However, this hook still
 * manages view-layer concerns that are NOT deprecated:
 * - Navigation state (centerDate, visibleDays)
 * - Flash animations for entry changes
 * - Toast notifications
 * - SSE event handlers (visual feedback only - connection managed by useSseWithOffline)
 *
 * For new calendars with offline support, use:
 * - useOfflineCalendar for data persistence
 * - useSseWithOffline for SSE handling (data updates)
 * - useInitialSync for hydration
 * - This hook for navigation, visual feedback, and SSE event handlers
 *
 * @see useOfflineCalendar
 * @see .claude/context/offline-first.md
 */

import { addDays, formatDateKey, isSameDay, parseDateKey } from "@core/calendar/dates"
import { mergeEntryCache, serverDaysToMap, updateEntryInCache } from "@core/calendar/entries"
import { calculateVisibleDays } from "@core/calendar/layout"
import type { ServerEntry } from "@core/calendar/types"
import type { SseConnectionState } from "@core/sse/types"
import { useTransport } from "@core/transport"
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react"

import type { CalendarActions, CalendarState, ChangeType, InitialData, ToastData } from "../types"

/** Number of days prefetched from server (before and after highlighted day) */
const PREFETCH_DAYS = 365

/** Buffer days to fetch when reaching edge of prefetched data */
const FETCH_BUFFER = 30

/** Duration to show flash animation (ms) - matches CSS animation */
const FLASH_DURATION = 1500

/** Duration to show toast notifications (ms) */
const TOAST_DURATION = 4000

/** Generate unique ID for toasts */
let toastIdCounter = 0
function generateToastId(): string {
  return `toast-${++toastIdCounter}-${Date.now()}`
}

/**
 * Calculate the number of days between two dates.
 */
function daysBetween(a: Date, b: Date): number {
  const msPerDay = 1000 * 60 * 60 * 24
  return Math.round((b.getTime() - a.getTime()) / msPerDay)
}

/** Configuration for useCalendarState hook */
export interface UseCalendarStateConfig {
  /** Initial data from SSR */
  initialData: InitialData
  /** Optional callback when entry changes occur (for notification center) */
  onNotification?: (type: ChangeType, entryId: string, entryTitle: string, date: string) => void
}

/**
 * Hook to manage calendar state.
 *
 * @param config - Hook configuration
 * @returns Tuple of [state, actions]
 */
export function useCalendarState(config: UseCalendarStateConfig): [CalendarState, CalendarActions] {
  const { initialData, onNotification } = config
  const transport = useTransport()

  // Parse initial highlighted day, correcting for server/client timezone mismatch
  const initialHighlightedDay = useMemo(() => {
    const serverDate = parseDateKey(initialData.highlightedDay)
    const clientToday = new Date()
    clientToday.setHours(0, 0, 0, 0)

    // If server date doesn't match client's today, check for timezone mismatch
    // A 1-day difference suggests the server tried to send "today" but got the wrong day
    if (!isSameDay(serverDate, clientToday)) {
      const dayDiff = Math.abs(daysBetween(serverDate, clientToday))
      if (dayDiff === 1) {
        // Likely timezone mismatch - use client's today
        return clientToday
      }
    }
    return serverDate
  }, [initialData.highlightedDay])

  // Initialize center date from initial data
  const [centerDate, setCenterDate] = useState<Date>(() => initialHighlightedDay)

  // Initialize entry cache from initial data
  const [entryCache, setEntryCache] = useState<Map<string, ServerEntry[]>>(() =>
    serverDaysToMap(initialData.days),
  )

  // Layout state - default to desktop view for SSR (7 days)
  const [visibleDays, setVisibleDays] = useState<number>(7)

  // SSE connection state (replaces loading states)
  const [sseConnectionState, setSseConnectionState] = useState<SseConnectionState>("disconnected")

  // Error state for API failures
  const [error, setError] = useState<string | null>(null)

  // Flash states for entry animations
  const [flashStates, setFlashStates] = useState<Map<string, ChangeType>>(() => new Map())

  // Toast notifications
  const [toasts, setToasts] = useState<ToastData[]>([])

  // AbortController ref for cancelling fetch requests
  const abortControllerRef = useRef<AbortController | null>(null)

  // Track fetched date ranges to avoid redundant fetches
  const fetchedRangesRef = useRef<{ start: Date; end: Date }[]>([
    {
      start: addDays(initialHighlightedDay, -PREFETCH_DAYS),
      end: addDays(initialHighlightedDay, PREFETCH_DAYS),
    },
  ])

  // Today's date (memoized, doesn't change)
  const today = useMemo(() => {
    const t = new Date()
    t.setHours(0, 0, 0, 0)
    return t
  }, [])

  /**
   * Add a flash state for an entry.
   */
  const addFlashState = useCallback((entryId: string, state: ChangeType) => {
    setFlashStates((prev) => {
      const next = new Map(prev)
      next.set(entryId, state)
      return next
    })

    // Auto-remove after animation completes
    setTimeout(() => {
      setFlashStates((prev) => {
        const next = new Map(prev)
        next.delete(entryId)
        return next
      })
    }, FLASH_DURATION)
  }, [])

  /**
   * Add a toast notification.
   */
  const addToast = useCallback((type: ChangeType, title: string, date: string) => {
    const id = generateToastId()
    const toast: ToastData = { id, type, title, date }

    setToasts((prev) => [...prev, toast])

    // Auto-remove after duration
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id))
    }, TOAST_DURATION)
  }, [])

  /**
   * Remove a toast notification.
   */
  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id))
  }, [])

  /**
   * Handle entry added event from SSE.
   */
  const handleEntryAdded = useCallback(
    (entry: ServerEntry, date: string) => {
      // Update cache
      setEntryCache((prev) => {
        const next = new Map(prev)
        const existing = next.get(date) || []
        // Avoid duplicates
        if (!existing.some((e) => e.id === entry.id)) {
          next.set(date, [...existing, entry])
        }
        return next
      })

      // Add flash animation and toast
      addFlashState(entry.id, "added")
      addToast("added", entry.title, date)

      // Notify notification center
      onNotification?.("added", entry.id, entry.title, date)
    },
    [addFlashState, addToast, onNotification],
  )

  /**
   * Handle entry updated event from SSE.
   */
  const handleEntryUpdated = useCallback(
    (entry: ServerEntry, date: string) => {
      // Update cache using pure helper function
      setEntryCache((prev) => updateEntryInCache(prev, entry))

      // Add flash animation and toast
      addFlashState(entry.id, "updated")
      addToast("updated", entry.title, date)

      // Notify notification center
      onNotification?.("updated", entry.id, entry.title, date)
    },
    [addFlashState, addToast, onNotification],
  )

  /**
   * Handle entry deleted event from SSE.
   */
  const handleEntryDeleted = useCallback(
    (entryId: string, date: string) => {
      // Find the entry title before deleting and trigger notifications
      setEntryCache((prev) => {
        const existing = prev.get(date) || []
        const entry = existing.find((e) => e.id === entryId)
        const deletedTitle = entry?.title || "Entry"

        // Add flash animation and toast (using captured title)
        addFlashState(entryId, "deleted")
        addToast("deleted", deletedTitle, date)

        // Notify notification center
        onNotification?.("deleted", entryId, deletedTitle, date)

        return prev // Don't modify cache yet
      })

      // Remove from cache after a brief delay to show animation
      setTimeout(() => {
        setEntryCache((prev) => {
          const next = new Map(prev)
          const existing = next.get(date) || []
          const filtered = existing.filter((e) => e.id !== entryId)
          if (filtered.length !== existing.length) {
            next.set(date, filtered)
          }
          return next
        })
      }, 1000) // Match flash-deleted animation duration
    },
    [addFlashState, addToast, onNotification],
  )

  /**
   * Handle SSE connection state change.
   */
  const handleConnectionChange = useCallback((state: SseConnectionState) => {
    setSseConnectionState(state)
  }, [])

  /**
   * Check if a date is within any fetched range.
   */
  const isDateFetched = useCallback((date: Date): boolean => {
    return fetchedRangesRef.current.some((range) => date >= range.start && date <= range.end)
  }, [])

  /**
   * Fetch entries for a date range and merge into cache.
   */
  const loadEntries = useCallback(
    async (highlightedDay: string, before: number, after: number) => {
      // Cancel any existing request
      if (abortControllerRef.current) {
        abortControllerRef.current.abort()
      }

      const controller = new AbortController()
      abortControllerRef.current = controller

      setError(null)

      try {
        const days = await transport.fetchEntries({
          calendarId: initialData.calendarId,
          highlightedDay,
          before,
          after,
          signal: controller.signal,
        })

        if (!controller.signal.aborted) {
          setEntryCache((prev) => mergeEntryCache(prev, days))

          // Track fetched range
          const centerDateForRange = parseDateKey(highlightedDay)
          fetchedRangesRef.current.push({
            start: addDays(centerDateForRange, -before),
            end: addDays(centerDateForRange, after),
          })
        }
      } catch (err) {
        if (err instanceof Error && err.name === "AbortError") {
          return
        }
        const message = err instanceof Error ? err.message : "Failed to load entries"
        setError(message)
        console.error("[Calendar] Failed to load entries:", err)
      }
    },
    [transport, initialData.calendarId],
  )

  /**
   * Check if we need to fetch more data and do so if necessary.
   */
  const ensureDataForDate = useCallback(
    (date: Date) => {
      // Check if date is within any fetched range
      if (isDateFetched(date)) {
        return
      }

      // Fetch data centered on this date
      const dateKey = formatDateKey(date)
      loadEntries(dateKey, FETCH_BUFFER, FETCH_BUFFER)
    },
    [isDateFetched, loadEntries],
  )

  /**
   * Update layout based on viewport width.
   */
  const updateLayout = useCallback((width: number) => {
    const newVisibleDays = calculateVisibleDays(width)
    setVisibleDays(newVisibleDays)
  }, [])

  /**
   * Navigate by a number of days.
   */
  const navigateDays = useCallback(
    (offset: number) => {
      setCenterDate((prev) => {
        const newDate = addDays(prev, offset)

        // Check if we need to load more data (approaching edge of prefetched range)
        const daysFromHighlighted = Math.abs(daysBetween(initialHighlightedDay, newDate))
        if (daysFromHighlighted > PREFETCH_DAYS - FETCH_BUFFER) {
          ensureDataForDate(newDate)
        }

        return newDate
      })
    },
    [initialHighlightedDay, ensureDataForDate],
  )

  /**
   * Jump to today.
   */
  const goToToday = useCallback(() => {
    setCenterDate(new Date(today))
    ensureDataForDate(today)
  }, [today, ensureDataForDate])

  /**
   * Jump to a specific date.
   */
  const goToDate = useCallback(
    (date: Date) => {
      setCenterDate(date)
      ensureDataForDate(date)
    },
    [ensureDataForDate],
  )

  /**
   * Add entry to cache optimistically (before SSE confirmation).
   * Used for immediate UI feedback when creating entries.
   * SSE handler will deduplicate when the server event arrives.
   */
  const addEntryOptimistic = useCallback((entry: ServerEntry) => {
    const date = entry.startDate
    setEntryCache((prev) => {
      const next = new Map(prev)
      const existing = next.get(date) || []
      // Avoid duplicates
      if (!existing.some((e) => e.id === entry.id)) {
        next.set(date, [...existing, entry])
      }
      return next
    })
  }, [])

  /**
   * Update entry in cache optimistically (before SSE confirmation).
   * Used for immediate UI feedback when updating entries.
   * SSE handler will reconcile if server data differs.
   */
  const updateEntryOptimistic = useCallback((entry: ServerEntry) => {
    // Update cache using pure helper function
    setEntryCache((prev) => updateEntryInCache(prev, entry))
  }, [])

  // Initialize layout on mount - useLayoutEffect ensures layout is calculated
  // before browser paint, avoiding flash of incorrect column count
  useLayoutEffect(() => {
    if (typeof window !== "undefined") {
      updateLayout(window.innerWidth)

      const handleResize = () => {
        updateLayout(window.innerWidth)
      }

      window.addEventListener("resize", handleResize)
      return () => window.removeEventListener("resize", handleResize)
    }
  }, [updateLayout])

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort()
      }
    }
  }, [])

  // Build state object
  const state: CalendarState = {
    centerDate,
    visibleDays,
    entryCache,
    sseConnectionState,
    error,
    flashStates,
    toasts,
  }

  // Build actions object
  const actions: CalendarActions = {
    navigateDays,
    goToToday,
    goToDate,
    updateLayout,
    removeToast,
    addEntryOptimistic,
    updateEntryOptimistic,
    // SSE event handlers with visual feedback (flash, toast, notification)
    onSseEntryAdded: handleEntryAdded,
    onSseEntryUpdated: handleEntryUpdated,
    onSseEntryDeleted: handleEntryDeleted,
    onSseConnectionChange: handleConnectionChange,
  }

  return [state, actions]
}

/**
 * Check if the center date is today.
 */
export function isOnToday(centerDate: Date): boolean {
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  return isSameDay(centerDate, today)
}
