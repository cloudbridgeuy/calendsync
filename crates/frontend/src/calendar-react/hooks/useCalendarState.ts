/**
 * Calendar state management hook.
 * Manages center date, visible days, entry cache, and SSE updates.
 *
 * With SSR prefetching 365 days in each direction, this hook:
 * - Connects to SSE for real-time entry updates
 * - Only fetches from API if user navigates beyond prefetched range
 * - Updates entry cache in response to SSE events
 * - Shows flash animations and toast notifications for changes
 */

import { addDays, formatDateKey, isSameDay } from "@core/calendar/dates"
import { mergeEntryCache, serverDaysToMap } from "@core/calendar/entries"
import { calculateVisibleDays, isMobileViewport } from "@core/calendar/layout"
import type { ServerEntry } from "@core/calendar/types"
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react"

import type {
    CalendarActions,
    CalendarState,
    FlashState,
    InitialData,
    NotificationType,
    SseConnectionState,
    ToastData,
} from "../types"
import { fetchEntries } from "./useApi"
import { useSse } from "./useSse"

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
 * Parse a date string (YYYY-MM-DD) to a Date object at midnight local time.
 */
function parseDateKey(dateKey: string): Date {
    const [year, month, day] = dateKey.split("-").map(Number)
    return new Date(year, month - 1, day)
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
    onNotification?: (
        type: NotificationType,
        entryId: string,
        entryTitle: string,
        date: string,
    ) => void
}

/**
 * Hook to manage calendar state.
 *
 * @param config - Hook configuration
 * @returns Tuple of [state, actions]
 */
export function useCalendarState(config: UseCalendarStateConfig): [CalendarState, CalendarActions] {
    const { initialData, onNotification } = config
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

    // Layout state - default to desktop view for SSR (7 days, not mobile)
    const [visibleDays, setVisibleDays] = useState<number>(7)
    const [isMobile, setIsMobile] = useState<boolean>(false)

    // SSE connection state (replaces loading states)
    const [sseConnectionState, setSseConnectionState] = useState<SseConnectionState>("disconnected")

    // Error state for API failures
    const [error, setError] = useState<string | null>(null)

    // Flash states for entry animations
    const [flashStates, setFlashStates] = useState<Map<string, FlashState>>(() => new Map())

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
    const addFlashState = useCallback((entryId: string, state: FlashState) => {
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
    const addToast = useCallback((type: FlashState, title: string, date: string) => {
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
            console.log("[Calendar] Entry added:", entry.title, "on", date)

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
            console.log("[Calendar] Entry updated:", entry.title, "on", date)

            // Update cache
            setEntryCache((prev) => {
                const next = new Map(prev)

                // First, remove the entry from any existing date (in case date changed)
                for (const [key, entries] of next.entries()) {
                    const filtered = entries.filter((e) => e.id !== entry.id)
                    if (filtered.length !== entries.length) {
                        next.set(key, filtered)
                    }
                }

                // Then add/update at the correct date
                const existing = next.get(date) || []
                const index = existing.findIndex((e) => e.id === entry.id)
                if (index >= 0) {
                    const updated = [...existing]
                    updated[index] = entry
                    next.set(date, updated)
                } else {
                    next.set(date, [...existing, entry])
                }

                return next
            })

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
            console.log("[Calendar] Entry deleted:", entryId, "on", date)

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

    // Setup SSE connection
    const { reconnect: reconnectSse } = useSse({
        calendarId: initialData.calendarId,
        onEntryAdded: handleEntryAdded,
        onEntryUpdated: handleEntryUpdated,
        onEntryDeleted: handleEntryDeleted,
        onConnectionChange: handleConnectionChange,
    })

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
                const days = await fetchEntries(
                    initialData.calendarId,
                    highlightedDay,
                    before,
                    after,
                    controller.signal,
                )

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
        [initialData.calendarId],
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
        const newIsMobile = isMobileViewport(width)
        setVisibleDays(newVisibleDays)
        setIsMobile(newIsMobile)
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
     * Refresh - reconnect SSE to get latest events.
     */
    const refresh = useCallback(async () => {
        reconnectSse()
    }, [reconnectSse])

    /**
     * Add entry to cache optimistically (before SSE confirmation).
     * Used for immediate UI feedback when creating entries.
     * SSE handler will deduplicate when the server event arrives.
     */
    const addEntryOptimistic = useCallback((entry: ServerEntry) => {
        const date = entry.date
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
        const date = entry.date
        setEntryCache((prev) => {
            const next = new Map(prev)

            // First, remove the entry from any existing date (in case date changed)
            for (const [key, entries] of next.entries()) {
                const filtered = entries.filter((e) => e.id !== entry.id)
                if (filtered.length !== entries.length) {
                    next.set(key, filtered)
                }
            }

            // Then add/update at the correct date
            const existing = next.get(date) || []
            const index = existing.findIndex((e) => e.id === entry.id)
            if (index >= 0) {
                const updated = [...existing]
                updated[index] = entry
                next.set(date, updated)
            } else {
                next.set(date, [...existing, entry])
            }

            return next
        })
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
        isMobile,
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
        refresh,
        updateLayout,
        removeToast,
        addEntryOptimistic,
        updateEntryOptimistic,
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
