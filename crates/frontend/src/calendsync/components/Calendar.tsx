/**
 * Main Calendar component - the entry point for the calendar UI.
 * Refactored as a compound component with unified horizontal navigation.
 */

import { addDays, formatDateKey, isSameCalendarDay } from "@core/calendar"
import { isAudioSupported, isVibrationSupported } from "@core/calendar/feedback"
import type { ServerEntry } from "@core/calendar/types"
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react"
import type { CalendarContextValue } from "../contexts"
import { CalendarProvider, useCalendarContext } from "../contexts"
import {
  useCalendarState,
  useEntryApi,
  useModalUrl,
  useNotificationCenter,
  useVirtualScroll,
} from "../hooks"
import type { InitialData } from "../types"
import { CalendarHeader } from "./CalendarHeader"
import { DayColumn } from "./DayColumn"
import { DayContainer } from "./DayContainer"
import { EntryModal } from "./EntryModal"
import { NotificationCenter } from "./NotificationCenter"
import { TodayButton } from "./TodayButton"

interface CalendarProps {
  initialData: InitialData
  children: React.ReactNode
}

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
  const { centerDate, entryCache, sseConnectionState, error, flashStates } = state

  // Modal URL state management
  const { modalState, openCreateModal, openEditModal, closeModal, closeAfterSave } = useModalUrl({
    calendarId: initialData.calendarId,
    initialModal: initialData.modal,
  })

  // Entry API for fetching entry data on client-side navigation
  const entryApi = useEntryApi({ calendarId: initialData.calendarId })

  // Track the entry being edited (from SSR, cache, or fetched)
  const [editEntry, setEditEntry] = useState<ServerEntry | undefined>(initialData.modal?.entry)

  // Track container width for dayWidth calculation
  const [containerWidth, setContainerWidth] = useState(0)
  const containerMeasureRef = useRef<HTMLDivElement>(null)

  // Measure container width on mount and resize
  useLayoutEffect(() => {
    const measure = () => {
      if (containerMeasureRef.current) {
        setContainerWidth(containerMeasureRef.current.offsetWidth)
      }
    }

    measure()
    window.addEventListener("resize", measure)
    return () => window.removeEventListener("resize", measure)
  }, [])

  // Audio context for navigation feedback
  const audioContextRef = useRef<AudioContext | null>(null)

  // Navigation feedback (haptic/audio)
  const triggerFeedback = useCallback(() => {
    // Vibration
    if (isVibrationSupported()) {
      navigator.vibrate(10)
    }
    // Sound (short click/tick)
    if (isAudioSupported()) {
      if (!audioContextRef.current) {
        audioContextRef.current = new (window.AudioContext || (window as any).webkitAudioContext)()
      }
      const ctx = audioContextRef.current
      const oscillator = ctx.createOscillator()
      const gain = ctx.createGain()
      oscillator.connect(gain)
      gain.connect(ctx.destination)
      oscillator.frequency.value = 1000 // 1kHz tick
      gain.gain.value = 0.1 // Low volume
      oscillator.start()
      oscillator.stop(ctx.currentTime + 0.01) // 10ms duration
    }
  }, [])

  // Virtual scroll hook for native scroll-based navigation
  const {
    scrollContainerRef,
    highlightedDate,
    renderedDates,
    dayWidth,
    visibleDays: virtualVisibleDays,
    scrollToDate,
    scrollToToday,
  } = useVirtualScroll({
    initialCenterDate: centerDate,
    containerWidth,
    enabled: true,
    onHighlightedDayChange: () => {
      actions.navigateDays(0) // Trigger any necessary data prefetch
    },
    onNavigationFeedback: triggerFeedback,
  })

  // Navigate by days using scrollToDate
  const navigateDays = useCallback(
    (days: number) => {
      const targetDate = addDays(highlightedDate, days)
      scrollToDate(targetDate)
    },
    [highlightedDate, scrollToDate],
  )

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
        navigateDays(-1)
      } else if (e.key === "ArrowRight") {
        navigateDays(1)
      } else if (e.key === "t" || e.key === "T") {
        scrollToToday()
      }
    }

    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [navigateDays, scrollToToday])

  // Build context value with useMemo
  const contextValue = useMemo<CalendarContextValue>(
    () => ({
      // Existing - entry display
      flashStates,
      onEntryClick: handleEntryClick,
      onEntryToggle: handleEntryToggle,
      // State
      centerDate,
      visibleDays: virtualVisibleDays,
      entryCache,
      sseConnectionState,
      error,
      // Virtual scroll state
      scrollContainerRef,
      highlightedDate,
      renderedDates,
      dayWidth,
      // Navigation actions
      scrollToDate,
      navigateDays,
      goToToday: scrollToToday,
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
      centerDate,
      virtualVisibleDays,
      entryCache,
      sseConnectionState,
      error,
      scrollContainerRef,
      highlightedDate,
      renderedDates,
      dayWidth,
      scrollToDate,
      navigateDays,
      scrollToToday,
      getEntriesForDate,
      actions,
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
      <div ref={containerMeasureRef} className="calendar-container">
        {children}
      </div>
    </CalendarProvider>
  )
}

/**
 * Header sub-component
 */
function Header() {
  const { highlightedDate } = useCalendarContext()
  return <CalendarHeader highlightedDate={highlightedDate} />
}

/**
 * NotificationCenter wrapper sub-component
 */
function NotificationCenterWrapper() {
  const { notificationState, notificationActions } = useCalendarContext()
  return (
    <div className="notification-center-container">
      <NotificationCenter state={notificationState} actions={notificationActions}>
        <NotificationCenter.Bell />
        <NotificationCenter.Panel>
          <NotificationCenter.Items />
        </NotificationCenter.Panel>
      </NotificationCenter>
    </div>
  )
}

/**
 * VirtualDaysContent - renders day columns from the virtual scroll window.
 * Uses native browser scroll for navigation.
 */
function VirtualDaysContent() {
  const { renderedDates, dayWidth, getEntriesForDate, highlightedDate, scrollToDate } =
    useCalendarContext()

  return (
    <>
      {renderedDates.map((date) => {
        const dateKey = formatDateKey(date)
        const entries = getEntriesForDate(date)
        const isHighlighted = isSameCalendarDay(date, highlightedDate)

        return (
          <DayContainer
            key={dateKey}
            date={date}
            dayWidth={dayWidth}
            isHighlighted={isHighlighted}
            onHeaderClick={() => scrollToDate(date)}
          >
            <DayContainer.Header />
            <DayContainer.Content>
              <DayColumn dateKey={dateKey} entries={entries} />
            </DayContainer.Content>
          </DayContainer>
        )
      })}
    </>
  )
}

/**
 * Days sub-component - scroll container for virtual scrolling.
 * Uses native browser scroll with hidden scrollbar.
 */
function Days() {
  const { sseConnectionState, error, refresh, scrollContainerRef } = useCalendarContext()

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

      {/* Scroll container for virtual scrolling */}
      <main ref={scrollContainerRef} className="entry-container scroll-container">
        <div className="days-scroll">
          <VirtualDaysContent />
        </div>
      </main>
    </>
  )
}

/**
 * TodayButton sub-component
 */
function TodayButtonWrapper() {
  const { highlightedDate, goToToday } = useCalendarContext()
  const today = new Date()
  const isOnTodayDate = highlightedDate.toDateString() === today.toDateString()

  if (isOnTodayDate) return null

  return <TodayButton visible={true} onClick={goToToday} />
}

/**
 * Fab sub-component
 */
function Fab() {
  const { highlightedDate, openCreateModal } = useCalendarContext()
  const dateKey = formatDateKey(highlightedDate)

  return (
    <button
      type="button"
      className="fab"
      onClick={() => openCreateModal(dateKey)}
      aria-label="Create new entry"
    >
      New
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
    highlightedDate,
  } = useCalendarContext()

  if (!modalState) return null

  return (
    <EntryModal
      mode={modalState.mode}
      entry={modalState.mode === "edit" ? editEntry : undefined}
      defaultDate={
        modalState.mode === "create"
          ? modalState.defaultDate || formatDateKey(highlightedDate)
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
