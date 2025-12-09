/**
 * Main Calendar component - the entry point for the calendar UI.
 * Refactored as a compound component.
 */

import { addDays, formatDateKey } from "@core/calendar/dates"
import type { ServerEntry } from "@core/calendar/types"
import { DEFAULT_LAYOUT_CONSTANTS } from "@core/calendar/types"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import type { CalendarContextValue } from "../contexts"
import { CalendarProvider, useCalendarContext } from "../contexts"
import { useCalendarState, useEntryApi, useModalUrl, useNotificationCenter } from "../hooks"
import type { InitialData } from "../types"
import { CalendarHeader } from "./CalendarHeader"
import { DayColumn } from "./DayColumn"
import { EntryModal } from "./EntryModal"
import { NotificationCenter } from "./NotificationCenter"
import { TodayButton } from "./TodayButton"

interface CalendarProps {
  initialData: InitialData
  children: React.ReactNode
}

const { swipeThreshold, velocityThreshold, mobileBuffer } = DEFAULT_LAYOUT_CONSTANTS

/**
 * CalendarRoot - Main Calendar component with state management.
 * This is the root of the compound component that manages all state and provides context.
 */
function CalendarRoot({ initialData, children }: CalendarProps) {
  // Notification center hook
  const [notificationState, notificationActions, addNotification] = useNotificationCenter({
    calendarId: initialData.calendarId,
  })

  // Calendar state with notification callback
  const [state, actions] = useCalendarState({
    initialData,
    onNotification: addNotification,
  })
  const { centerDate, visibleDays, isMobile, entryCache, sseConnectionState, error, flashStates } =
    state

  // Modal URL state management
  const { modalState, openCreateModal, openEditModal, closeModal, closeAfterSave } = useModalUrl({
    calendarId: initialData.calendarId,
    initialModal: initialData.modal,
  })

  // Entry API for fetching entry data on client-side navigation
  const entryApi = useEntryApi({ calendarId: initialData.calendarId })

  // Track the entry being edited (from SSR, cache, or fetched)
  const [editEntry, setEditEntry] = useState<ServerEntry | undefined>(initialData.modal?.entry)

  // Fetch entry data when navigating to edit URL on client side
  useEffect(() => {
    if (modalState?.mode === "edit" && modalState.entryId && !editEntry) {
      // First try to get from cache
      for (const entries of entryCache.values()) {
        const found = entries.find((e) => e.id === modalState.entryId)
        if (found) {
          setEditEntry(found)
          return
        }
      }

      // If not in cache, fetch from API
      entryApi
        .fetchEntry(modalState.entryId)
        .then((entry) => setEditEntry(entry))
        .catch(() => {
          // Entry not found, close modal
          closeModal()
        })
    }
  }, [modalState, editEntry, entryCache, entryApi, closeModal])

  // Clear edit entry when modal closes
  useEffect(() => {
    if (!modalState) {
      setEditEntry(undefined)
    }
  }, [modalState])

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
   * Handle entry click to open edit modal.
   */
  const handleEntryClick = useCallback(
    (entry: ServerEntry) => {
      setEditEntry(entry) // Pre-populate from cache
      openEditModal(entry.id)
    },
    [openEditModal],
  )

  /**
   * Handle task toggle - optimistic update with API call.
   */
  const handleEntryToggle = useCallback(
    (entry: ServerEntry) => {
      // Optimistic update: immediately toggle completed state
      const toggledEntry = { ...entry, completed: !entry.completed }
      actions.updateEntryOptimistic(toggledEntry)

      // Call API in background
      entryApi.toggleEntry(entry.id).catch(() => {
        // On error, revert to original state
        actions.updateEntryOptimistic(entry)
      })
    },
    [actions, entryApi],
  )

  /**
   * Handle modal save - apply optimistic update immediately.
   * SSE will confirm/reconcile when the server event arrives.
   */
  const handleModalSave = useCallback(
    (savedEntry: ServerEntry) => {
      // Apply optimistic update immediately for instant feedback
      if (modalState?.mode === "create") {
        actions.addEntryOptimistic(savedEntry)
      } else {
        actions.updateEntryOptimistic(savedEntry)
      }
      closeAfterSave()
    },
    [modalState, actions, closeAfterSave],
  )

  /**
   * Handle modal delete.
   */
  const handleModalDelete = useCallback(() => {
    // Note: We don't do optimistic delete here - the SSE event handles removal
    // This ensures proper animation timing
    closeAfterSave()
  }, [closeAfterSave])

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
   * Handle wheel navigation with direction lock.
   * - Horizontal scroll (deltaX): Navigate days (no modifier needed)
   * - Vertical scroll (deltaY) + Cmd/Ctrl: Navigate days
   * - Vertical scroll (deltaY) without modifier: Default (scroll entries)
   *
   * Direction lock: Once a gesture direction is detected (horizontal or vertical),
   * all events in the other direction are ignored until the gesture ends.
   */
  useEffect(() => {
    // Accumulator state for trackpad gestures
    let accumulatedDeltaX = 0
    let accumulatedDeltaY = 0
    let lastWheelTime = 0

    // Direction lock state: null = not locked, 'x' = horizontal, 'y' = vertical
    let lockedDirection: "x" | "y" | null = null

    const TRACKPAD_THRESHOLD = 10 // Pixels per day for trackpad
    const MOUSE_WHEEL_THRESHOLD = 50 // Threshold to detect mouse wheel vs trackpad
    const GESTURE_TIMEOUT = 50 // ms - reset accumulator and direction lock if no events
    const DIRECTION_LOCK_THRESHOLD = 1 // Pixels to determine initial direction

    const handleWheel = (e: WheelEvent) => {
      const now = Date.now()
      const timeSinceLast = now - lastWheelTime

      // Reset state if gesture ended (time gap)
      if (timeSinceLast > GESTURE_TIMEOUT) {
        accumulatedDeltaX = 0
        accumulatedDeltaY = 0
        lockedDirection = null
      }
      lastWheelTime = now

      // Determine direction lock on first significant movement
      if (lockedDirection === null) {
        if (Math.abs(e.deltaX) >= DIRECTION_LOCK_THRESHOLD) {
          lockedDirection = "x"
        } else if (Math.abs(e.deltaY) >= DIRECTION_LOCK_THRESHOLD) {
          lockedDirection = "y"
        }
      }

      const hasModifier = e.metaKey || e.ctrlKey

      // Handle based on locked direction
      if (lockedDirection === "x") {
        // Horizontal scroll - navigate days
        if (e.deltaX === 0) return

        e.preventDefault()

        const isMouseWheel = Math.abs(e.deltaX) >= MOUSE_WHEEL_THRESHOLD

        if (isMouseWheel) {
          const direction = e.deltaX > 0 ? 1 : -1
          actions.navigateDays(direction)
        } else {
          accumulatedDeltaX += e.deltaX

          while (Math.abs(accumulatedDeltaX) >= TRACKPAD_THRESHOLD) {
            const direction = accumulatedDeltaX > 0 ? 1 : -1
            actions.navigateDays(direction)
            accumulatedDeltaX -= direction * TRACKPAD_THRESHOLD
          }
        }
      } else if (lockedDirection === "y") {
        // Vertical scroll
        if (e.deltaY === 0) return

        // Only handle vertical with modifier, otherwise let browser scroll
        if (!hasModifier) return

        e.preventDefault()

        const isMouseWheel = Math.abs(e.deltaY) >= MOUSE_WHEEL_THRESHOLD

        if (isMouseWheel) {
          const direction = e.deltaY > 0 ? 1 : -1
          actions.navigateDays(direction)
        } else {
          accumulatedDeltaY += e.deltaY

          while (Math.abs(accumulatedDeltaY) >= TRACKPAD_THRESHOLD) {
            const direction = accumulatedDeltaY > 0 ? 1 : -1
            actions.navigateDays(direction)
            accumulatedDeltaY -= direction * TRACKPAD_THRESHOLD
          }
        }
      }
      // If no direction locked yet, don't handle (wait for threshold)
    }

    window.addEventListener("wheel", handleWheel, { passive: false })
    return () => window.removeEventListener("wheel", handleWheel)
  }, [actions])

  // Build context value with useMemo
  const contextValue = useMemo<CalendarContextValue>(
    () => ({
      // Existing - entry display
      flashStates,
      onEntryClick: handleEntryClick,
      onEntryToggle: handleEntryToggle,
      isMobile,
      // State
      centerDate,
      visibleDays,
      entryCache,
      sseConnectionState,
      error,
      // Navigation actions
      navigateDays: actions.navigateDays,
      goToToday: actions.goToToday,
      getEntriesForDate,
      refresh: actions.refresh,
      // Modal state
      openCreateModal,
      openEditModal,
      closeModal,
      modalState,
      editEntry,
      handleModalSave,
      handleModalDelete,
      calendarId: initialData.calendarId,
      // Notification state
      notificationState,
      notificationActions,
      addNotification,
    }),
    [
      flashStates,
      handleEntryClick,
      handleEntryToggle,
      isMobile,
      centerDate,
      visibleDays,
      entryCache,
      sseConnectionState,
      error,
      actions,
      getEntriesForDate,
      openCreateModal,
      openEditModal,
      closeModal,
      modalState,
      editEntry,
      handleModalSave,
      handleModalDelete,
      initialData.calendarId,
      notificationState,
      notificationActions,
      addNotification,
    ],
  )

  return (
    <CalendarProvider value={contextValue}>
      <div className="calendar-container">{children}</div>
    </CalendarProvider>
  )
}

/**
 * Header sub-component
 */
function Header() {
  const { centerDate, visibleDays, isMobile, navigateDays } = useCalendarContext()
  return (
    <CalendarHeader
      centerDate={centerDate}
      visibleDays={visibleDays}
      isMobile={isMobile}
      onNavigate={navigateDays}
    />
  )
}

/**
 * NotificationCenter wrapper sub-component
 */
function NotificationCenterWrapper() {
  const { notificationState, notificationActions, isMobile } = useCalendarContext()
  return (
    <div className="notification-center-container">
      <NotificationCenter
        state={notificationState}
        actions={notificationActions}
        isMobile={isMobile}
      >
        <NotificationCenter.Bell />
        <NotificationCenter.Panel>
          <NotificationCenter.Items />
        </NotificationCenter.Panel>
      </NotificationCenter>
    </div>
  )
}

/**
 * MobileDays internal component - contains all mobile swipe logic
 */
function MobileDays() {
  const { centerDate, getEntriesForDate, navigateDays } = useCalendarContext()
  const [isDragging, setIsDragging] = useState(false)
  const [dragOffset, setDragOffset] = useState(0)
  const touchStartRef = useRef<{ x: number; y: number; time: number } | null>(null)
  const isHorizontalSwipeRef = useRef<boolean | null>(null)

  /**
   * Touch start handler.
   */
  const handleTouchStart = useCallback((e: React.TouchEvent) => {
    const touch = e.touches[0]
    touchStartRef.current = {
      x: touch.clientX,
      y: touch.clientY,
      time: Date.now(),
    }
    isHorizontalSwipeRef.current = null
    setIsDragging(true)
  }, [])

  /**
   * Touch move handler.
   */
  const handleTouchMove = useCallback((e: React.TouchEvent) => {
    if (!touchStartRef.current) return

    const touch = e.touches[0]
    const diffX = touch.clientX - touchStartRef.current.x
    const diffY = touch.clientY - touchStartRef.current.y

    // Determine swipe direction on first significant movement
    if (isHorizontalSwipeRef.current === null && (Math.abs(diffX) > 10 || Math.abs(diffY) > 10)) {
      isHorizontalSwipeRef.current = Math.abs(diffX) > Math.abs(diffY)
    }

    // Only handle horizontal swipes
    if (isHorizontalSwipeRef.current) {
      e.preventDefault()
      const dragPercent = (diffX / window.innerWidth) * 100
      setDragOffset(dragPercent)
    }
  }, [])

  /**
   * Touch end handler.
   */
  const handleTouchEnd = useCallback(() => {
    if (!touchStartRef.current) {
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
        navigateDays(dayOffset)
        // Set starting position for incoming day's snap animation
        setDragOffset(dayOffset > 0 ? 100 : -100)

        // Use setTimeout to ensure React renders the new position first
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
  }, [dragOffset, navigateDays])

  // Render buffer days around center
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

  return (
    <main className="entry-container">
      {/* Scroll indicators */}
      <div className="scroll-indicator left">‹</div>
      <div className="scroll-indicator right">›</div>

      {/* Days scroll container */}
      <div
        className={`days-scroll${isDragging ? " dragging" : ""}`}
        onTouchStart={handleTouchStart}
        onTouchMove={handleTouchMove}
        onTouchEnd={handleTouchEnd}
        onTouchCancel={handleTouchEnd}
      >
        {columns}
      </div>

      {/* Navigation hint */}
      <div className="nav-hint">Swipe to navigate days</div>
    </main>
  )
}

/**
 * DesktopDays internal component - contains desktop layout
 */
function DesktopDays() {
  const { centerDate, visibleDays, getEntriesForDate, navigateDays } = useCalendarContext()

  /**
   * Handle wheel navigation with direction lock.
   */
  useEffect(() => {
    let accumulatedDeltaX = 0
    let accumulatedDeltaY = 0
    let lastWheelTime = 0
    let lockedDirection: "x" | "y" | null = null

    const TRACKPAD_THRESHOLD = 10
    const MOUSE_WHEEL_THRESHOLD = 50
    const GESTURE_TIMEOUT = 50
    const DIRECTION_LOCK_THRESHOLD = 1

    const handleWheel = (e: WheelEvent) => {
      const now = Date.now()
      const timeSinceLast = now - lastWheelTime

      if (timeSinceLast > GESTURE_TIMEOUT) {
        accumulatedDeltaX = 0
        accumulatedDeltaY = 0
        lockedDirection = null
      }
      lastWheelTime = now

      if (lockedDirection === null) {
        if (Math.abs(e.deltaX) >= DIRECTION_LOCK_THRESHOLD) {
          lockedDirection = "x"
        } else if (Math.abs(e.deltaY) >= DIRECTION_LOCK_THRESHOLD) {
          lockedDirection = "y"
        }
      }

      const hasModifier = e.metaKey || e.ctrlKey

      if (lockedDirection === "x") {
        if (e.deltaX === 0) return
        e.preventDefault()

        const isMouseWheel = Math.abs(e.deltaX) >= MOUSE_WHEEL_THRESHOLD

        if (isMouseWheel) {
          const direction = e.deltaX > 0 ? 1 : -1
          navigateDays(direction)
        } else {
          accumulatedDeltaX += e.deltaX

          while (Math.abs(accumulatedDeltaX) >= TRACKPAD_THRESHOLD) {
            const direction = accumulatedDeltaX > 0 ? 1 : -1
            navigateDays(direction)
            accumulatedDeltaX -= direction * TRACKPAD_THRESHOLD
          }
        }
      } else if (lockedDirection === "y") {
        if (e.deltaY === 0) return
        if (!hasModifier) return

        e.preventDefault()

        const isMouseWheel = Math.abs(e.deltaY) >= MOUSE_WHEEL_THRESHOLD

        if (isMouseWheel) {
          const direction = e.deltaY > 0 ? 1 : -1
          navigateDays(direction)
        } else {
          accumulatedDeltaY += e.deltaY

          while (Math.abs(accumulatedDeltaY) >= TRACKPAD_THRESHOLD) {
            const direction = accumulatedDeltaY > 0 ? 1 : -1
            navigateDays(direction)
            accumulatedDeltaY -= direction * TRACKPAD_THRESHOLD
          }
        }
      }
    }

    window.addEventListener("wheel", handleWheel, { passive: false })
    return () => window.removeEventListener("wheel", handleWheel)
  }, [navigateDays])

  // Render visible days centered on centerDate
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
        isLastVisible={isLastVisible}
        style={{
          width: `${columnWidth}%`,
          flexBasis: `${columnWidth}%`,
          flexShrink: 0,
        }}
      />,
    )
  }

  return (
    <main className="entry-container">
      <div className="days-scroll">{columns}</div>
    </main>
  )
}

/**
 * Days sub-component - renders either mobile or desktop layout
 */
function Days() {
  const { isMobile, sseConnectionState, error, refresh } = useCalendarContext()

  return (
    <>
      {/* SSE connection indicator */}
      {sseConnectionState === "connecting" && (
        <div className="sse-indicator connecting">Connecting...</div>
      )}
      {sseConnectionState === "error" && (
        <div className="sse-indicator error">
          Connection lost
          <button type="button" onClick={refresh}>
            Reconnect
          </button>
        </div>
      )}

      {/* Error message */}
      {error && (
        <div className="error-message">
          <div className="error-icon">⚠️</div>
          <div className="error-text">{error}</div>
          <button type="button" className="retry-button" onClick={refresh}>
            Retry
          </button>
        </div>
      )}

      {isMobile ? <MobileDays /> : <DesktopDays />}
    </>
  )
}

/**
 * TodayButton sub-component
 */
function TodayButtonWrapper() {
  const { centerDate, goToToday } = useCalendarContext()
  const today = new Date()
  const isOnTodayDate = centerDate.toDateString() === today.toDateString()

  if (isOnTodayDate) return null

  return <TodayButton visible={true} onClick={goToToday} />
}

/**
 * Fab sub-component
 */
function Fab() {
  const { centerDate, openCreateModal } = useCalendarContext()
  const dateKey = formatDateKey(centerDate)

  return (
    <button
      type="button"
      className="fab"
      onClick={() => openCreateModal(dateKey)}
      aria-label="Create new entry"
    >
      <span className="fab-icon">+</span>
    </button>
  )
}

/**
 * Modal sub-component
 */
function Modal() {
  const {
    modalState,
    closeModal,
    editEntry,
    handleModalSave,
    handleModalDelete,
    calendarId,
    centerDate,
  } = useCalendarContext()

  if (!modalState) return null

  return (
    <EntryModal
      mode={modalState.mode}
      entry={modalState.mode === "edit" ? editEntry : undefined}
      defaultDate={
        modalState.mode === "create"
          ? modalState.defaultDate || formatDateKey(centerDate)
          : undefined
      }
      calendarId={calendarId}
      onClose={closeModal}
      onSave={handleModalSave}
      onDelete={handleModalDelete}
    />
  )
}

/**
 * Calendar compound component with sub-components attached
 */
export const Calendar = Object.assign(CalendarRoot, {
  Header,
  NotificationCenter: NotificationCenterWrapper,
  Days,
  TodayButton: TodayButtonWrapper,
  Fab,
  Modal,
})
