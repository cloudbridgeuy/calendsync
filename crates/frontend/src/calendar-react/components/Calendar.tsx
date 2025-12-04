/**
 * Main Calendar component - the entry point for the calendar UI.
 */

import { addDays, formatDateKey } from "@core/calendar/dates"
import type { ServerEntry } from "@core/calendar/types"
import { DEFAULT_LAYOUT_CONSTANTS } from "@core/calendar/types"
import { useCallback, useEffect, useRef, useState } from "react"
import { isOnToday, useCalendarState, useNotificationCenter } from "../hooks"
import type { InitialData } from "../types"
import { CalendarHeader } from "./CalendarHeader"
import { DayColumn } from "./DayColumn"
import { NotificationCenter } from "./NotificationCenter"
import { TodayButton } from "./TodayButton"

interface CalendarProps {
    initialData: InitialData
}

const { swipeThreshold, velocityThreshold, mobileBuffer } = DEFAULT_LAYOUT_CONSTANTS

/**
 * Main Calendar component.
 */
export function Calendar({ initialData }: CalendarProps) {
    // Notification center hook
    const [notificationState, notificationActions, addNotification] = useNotificationCenter({
        calendarId: initialData.calendarId,
    })

    // Calendar state with notification callback
    const [state, actions] = useCalendarState({
        initialData,
        onNotification: addNotification,
    })
    const {
        centerDate,
        visibleDays,
        isMobile,
        entryCache,
        sseConnectionState,
        error,
        flashStates,
    } = state

    // Touch/swipe state
    const [isDragging, setIsDragging] = useState(false)
    const [dragOffset, setDragOffset] = useState(0)
    const touchStartRef = useRef<{ x: number; y: number; time: number } | null>(null)
    const isHorizontalSwipeRef = useRef<boolean | null>(null)

    // Refs for DOM elements
    const daysScrollRef = useRef<HTMLDivElement>(null)

    /**
     * Get entries for a specific date from the cache.
     */
    const getEntriesForDate = useCallback(
        (date: Date): ServerEntry[] => {
            const key = formatDateKey(date)
            return entryCache.get(key) || []
        },
        [entryCache],
    )

    /**
     * Handle keyboard navigation.
     */
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === "ArrowLeft") {
                actions.navigateDays(-1)
            } else if (e.key === "ArrowRight") {
                actions.navigateDays(1)
            } else if (e.key === "t" || e.key === "T") {
                actions.goToToday()
            }
        }

        window.addEventListener("keydown", handleKeyDown)
        return () => window.removeEventListener("keydown", handleKeyDown)
    }, [actions])

    /**
     * Handle wheel navigation (Cmd+Scroll or Ctrl+Scroll).
     * Detects mouse wheel vs trackpad and handles each appropriately:
     * - Mouse wheel: 1 event = 1 day (direct mapping)
     * - Trackpad: threshold-based accumulation for smooth scrolling
     */
    useEffect(() => {
        // Device detection state (undefined = not yet determined)
        let isTouchPad: boolean | undefined
        let eventCount = 0
        let eventCountStart = 0

        // Trackpad accumulation state
        let accumulatedDelta = 0

        const TRACKPAD_THRESHOLD = 30 // Pixels per day for trackpad

        const handleWheel = (e: WheelEvent) => {
            // Only handle if Cmd (Mac) or Ctrl (Windows/Linux) is held
            if (!(e.metaKey || e.ctrlKey)) return

            e.preventDefault()

            // === Device Detection Phase ===
            // Count events in first 50ms to detect device type
            // Mouse wheel: ≤5 events (discrete clicks)
            // Trackpad: >5 events (continuous touch)
            if (isTouchPad === undefined) {
                if (eventCount === 0) {
                    eventCountStart = Date.now()
                }
                eventCount++

                // Accumulate delta during detection (for trackpad case)
                accumulatedDelta += e.deltaY

                // After 50ms, determine device type
                if (Date.now() - eventCountStart > 50) {
                    isTouchPad = eventCount > 5

                    // If mouse wheel, process the accumulated delta as single navigation
                    if (!isTouchPad) {
                        const direction = accumulatedDelta > 0 ? 1 : -1
                        actions.navigateDays(direction)
                        accumulatedDelta = 0
                    }
                }
                return
            }

            // === Handle Based on Device Type ===
            if (isTouchPad) {
                // Trackpad: accumulate and navigate when threshold crossed
                // No direction change detection - let the accumulator naturally handle
                // reversals (opposite deltas will reduce the accumulator)
                accumulatedDelta += e.deltaY

                // Navigate when threshold crossed (use while to handle large deltas)
                while (Math.abs(accumulatedDelta) >= TRACKPAD_THRESHOLD) {
                    const direction = accumulatedDelta > 0 ? 1 : -1
                    actions.navigateDays(direction)
                    accumulatedDelta -= direction * TRACKPAD_THRESHOLD
                }
            } else {
                // Mouse wheel: 1 event = 1 day (simple, direct)
                const direction = e.deltaY > 0 ? 1 : -1
                actions.navigateDays(direction)
            }
        }

        window.addEventListener("wheel", handleWheel, { passive: false })
        return () => window.removeEventListener("wheel", handleWheel)
    }, [actions])

    /**
     * Touch start handler.
     */
    const handleTouchStart = useCallback(
        (e: React.TouchEvent) => {
            if (!isMobile) return

            const touch = e.touches[0]
            touchStartRef.current = {
                x: touch.clientX,
                y: touch.clientY,
                time: Date.now(),
            }
            isHorizontalSwipeRef.current = null
            setIsDragging(true)
        },
        [isMobile],
    )

    /**
     * Touch move handler.
     */
    const handleTouchMove = useCallback(
        (e: React.TouchEvent) => {
            if (!isMobile || !touchStartRef.current) return

            const touch = e.touches[0]
            const diffX = touch.clientX - touchStartRef.current.x
            const diffY = touch.clientY - touchStartRef.current.y

            // Determine swipe direction on first significant movement
            if (
                isHorizontalSwipeRef.current === null &&
                (Math.abs(diffX) > 10 || Math.abs(diffY) > 10)
            ) {
                isHorizontalSwipeRef.current = Math.abs(diffX) > Math.abs(diffY)
            }

            // Only handle horizontal swipes
            if (isHorizontalSwipeRef.current) {
                e.preventDefault()
                const dragPercent = (diffX / window.innerWidth) * 100
                setDragOffset(dragPercent)
            }
        },
        [isMobile],
    )

    /**
     * Touch end handler.
     */
    const handleTouchEnd = useCallback(() => {
        if (!isMobile || !touchStartRef.current) {
            setIsDragging(false)
            setDragOffset(0)
            return
        }

        const wasHorizontal = isHorizontalSwipeRef.current

        if (wasHorizontal) {
            const diffX = (dragOffset * window.innerWidth) / 100
            const elapsed = Date.now() - touchStartRef.current.time
            const velocity = Math.abs(diffX) / elapsed

            let dayOffset = 0
            if (Math.abs(diffX) > swipeThreshold || velocity > velocityThreshold) {
                dayOffset = diffX > 0 ? -1 : 1
            }

            if (dayOffset !== 0) {
                actions.navigateDays(dayOffset)
                // Set starting position for incoming day's snap animation
                // Going forward (+1): new day comes from right, so start at +100%
                // Going backward (-1): new day comes from left, so start at -100%
                setDragOffset(dayOffset > 0 ? 100 : -100)

                // Use setTimeout to ensure React renders the new position first,
                // then animate to 0
                setTimeout(() => {
                    setIsDragging(false)
                    setDragOffset(0)
                }, 0)
            } else {
                // Snap back to current day
                setIsDragging(false)
                setDragOffset(0)
            }
        } else {
            setIsDragging(false)
            setDragOffset(0)
        }

        touchStartRef.current = null
        isHorizontalSwipeRef.current = null
    }, [isMobile, dragOffset, actions])

    /**
     * Render day columns.
     */
    const renderDayColumns = () => {
        if (isMobile) {
            // Mobile: Render buffer days around center
            const totalDays = mobileBuffer * 2 + 1
            const columns = []

            for (let i = 0; i < totalDays; i++) {
                const offset = i - mobileBuffer
                const date = addDays(centerDate, offset)
                const dateKey = formatDateKey(date)
                const entries = getEntriesForDate(date)

                // Calculate transform for swipe animation
                const baseOffset = -mobileBuffer * 100
                const transform = `translateX(${baseOffset + dragOffset}%)`

                columns.push(
                    <DayColumn
                        key={dateKey}
                        dateKey={dateKey}
                        entries={entries}
                        flashStates={flashStates}
                        style={{
                            width: "100%",
                            flexBasis: "100%",
                            flexShrink: 0,
                            transition: isDragging ? "none" : "transform 0.2s ease-out",
                            transform,
                        }}
                    />,
                )
            }

            return columns
        }

        // Desktop: Render visible days centered on centerDate
        const halfDays = Math.floor(visibleDays / 2)
        const columnWidth = 100 / visibleDays
        const columns = []

        for (let i = 0; i < visibleDays; i++) {
            const date = addDays(centerDate, i - halfDays)
            const dateKey = formatDateKey(date)
            const entries = getEntriesForDate(date)
            const isLastVisible = i === visibleDays - 1

            columns.push(
                <DayColumn
                    key={dateKey}
                    dateKey={dateKey}
                    entries={entries}
                    flashStates={flashStates}
                    isLastVisible={isLastVisible}
                    style={{
                        width: `${columnWidth}%`,
                        flexBasis: `${columnWidth}%`,
                        flexShrink: 0,
                    }}
                />,
            )
        }

        return columns
    }

    // Show/hide today button
    const showTodayButton = !isOnToday(centerDate)

    return (
        <div className="calendar-container">
            <CalendarHeader
                centerDate={centerDate}
                visibleDays={visibleDays}
                isMobile={isMobile}
                onNavigate={actions.navigateDays}
            />

            {/* Notification center - positioned absolutely in header */}
            <div className="notification-center-container">
                <NotificationCenter
                    state={notificationState}
                    actions={notificationActions}
                    isMobile={isMobile}
                />
            </div>

            <main className="entry-container">
                {/* Scroll indicators (mobile only) */}
                {isMobile && (
                    <>
                        <div className="scroll-indicator left">‹</div>
                        <div className="scroll-indicator right">›</div>
                    </>
                )}

                {/* SSE connection indicator */}
                {sseConnectionState === "connecting" && (
                    <div className="sse-indicator connecting">Connecting...</div>
                )}
                {sseConnectionState === "error" && (
                    <div className="sse-indicator error">
                        Connection lost
                        <button type="button" onClick={actions.refresh}>
                            Reconnect
                        </button>
                    </div>
                )}

                {/* Error message */}
                {error && (
                    <div className="error-message">
                        <div className="error-icon">⚠️</div>
                        <div className="error-text">{error}</div>
                        <button type="button" className="retry-button" onClick={actions.refresh}>
                            Retry
                        </button>
                    </div>
                )}

                {/* Days scroll container */}
                <div
                    ref={daysScrollRef}
                    className={`days-scroll${isDragging ? " dragging" : ""}`}
                    onTouchStart={handleTouchStart}
                    onTouchMove={handleTouchMove}
                    onTouchEnd={handleTouchEnd}
                    onTouchCancel={handleTouchEnd}
                >
                    {renderDayColumns()}
                </div>
            </main>

            {/* Today button */}
            <TodayButton visible={showTodayButton} onClick={actions.goToToday} />

            {/* Navigation hint (mobile only, shown briefly) */}
            {isMobile && <div className="nav-hint">Swipe to navigate days</div>}
        </div>
    )
}
