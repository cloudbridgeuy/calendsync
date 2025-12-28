/**
 * Main Calendar component - the entry point for the calendar UI.
 * Refactored as a compound component with unified horizontal navigation.
 */

import {
  addDays,
  calculateScrollToHour,
  DEFAULT_SCROLL_HOUR,
  filterByTaskVisibility,
  formatDateKey,
  isSameCalendarDay,
} from "@core/calendar"
import type { ServerEntry } from "@core/calendar/types"
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react"
import type { CalendarContextValue } from "../contexts"
import { CalendarProvider, useCalendarContext } from "../contexts"
import {
  useCalendarSettings,
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
import { ScheduleGrid } from "./ScheduleGrid"
import { SettingsMenu } from "./SettingsMenu"
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
  // Settings hook
  const [settingsState, settingsActions] = useCalendarSettings({
    calendarId: initialData.calendarId,
  })

  // Notification center hook
  const [notificationState, notificationActions, addNotification] = useNotificationCenter({
    calendarId: initialData.calendarId,
  })

  // All-day section toggle states
  const [showAllDayOverflow, setShowAllDayOverflow] = useState(false)
  const [showAllDayTasks, setShowAllDayTasks] = useState(false)

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
   * Applies task visibility filtering based on settings.
   */
  const getEntriesForDate = useCallback(
    (date: Date): ServerEntry[] => {
      const key = formatDateKey(date)
      const entries = entryCache.get(key) || []
      return filterByTaskVisibility(entries, settingsState.showTasks)
    },
    [entryCache, settingsState.showTasks],
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
      // Settings state
      settings: settingsState,
      setViewMode: settingsActions.setViewMode,
      setShowTasks: settingsActions.setShowTasks,
      toggleShowTasks: settingsActions.toggleShowTasks,
      // All-day section toggle states
      showAllDayOverflow,
      setShowAllDayOverflow,
      showAllDayTasks,
      setShowAllDayTasks,
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
      settingsState,
      settingsActions,
      showAllDayOverflow,
      showAllDayTasks,
    ],
  )

  return (
    <CalendarProvider value={contextValue}>
      <div
        ref={containerMeasureRef}
        className={`calendar-container ${settingsState.viewMode}-mode`}
      >
        {children}
      </div>
    </CalendarProvider>
  )
}

/**
 * SettingsMenu wrapper sub-component
 */
function SettingsMenuWrapper() {
  const { settings, setViewMode, toggleShowTasks } = useCalendarContext()

  return (
    <SettingsMenu
      viewMode={settings.viewMode}
      showTasks={settings.showTasks}
      onViewModeChange={setViewMode}
      onToggleShowTasks={toggleShowTasks}
    >
      <SettingsMenu.Trigger />
      <SettingsMenu.Panel>
        <SettingsMenu.ViewToggle />
        <SettingsMenu.TaskToggle />
      </SettingsMenu.Panel>
    </SettingsMenu>
  )
}

/**
 * Header sub-component
 */
function Header() {
  const { highlightedDate } = useCalendarContext()
  return <CalendarHeader highlightedDate={highlightedDate} settingsSlot={<SettingsMenuWrapper />} />
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
  const { renderedDates, dayWidth, getEntriesForDate, highlightedDate, scrollToDate, settings } =
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
            isHighlighted={isHighlighted}
            onHeaderClick={() => scrollToDate(date)}
          >
            <DayContainer.Header />
            <DayContainer.Content>
              <DayColumn
                dateKey={dateKey}
                entries={entries}
                viewMode={settings.viewMode}
                dayWidth={dayWidth}
              />
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
  const { sseConnectionState, error, refresh, scrollContainerRef, settings } = useCalendarContext()
  const prevViewModeRef = useRef(settings.viewMode)
  const isScheduleMode = settings.viewMode === "schedule"

  // Scroll to 8 AM when switching to schedule mode (account for day header height)
  useEffect(() => {
    if (settings.viewMode === "schedule" && prevViewModeRef.current !== "schedule") {
      const scrollTop = calculateScrollToHour(DEFAULT_SCROLL_HOUR)
      scrollContainerRef.current?.scrollTo({ top: scrollTop })
    }
    prevViewModeRef.current = settings.viewMode
  }, [settings.viewMode, scrollContainerRef])

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
        {isScheduleMode ? (
          <ScheduleGrid>
            <ScheduleGrid.Corner />
            <ScheduleGrid.DayHeaders />
            <ScheduleGrid.AllDayLabel />
            <ScheduleGrid.AllDayEvents />
            <ScheduleGrid.HourColumn />
            <ScheduleGrid.TimedGrid />
          </ScheduleGrid>
        ) : (
          <div className="days-scroll">
            <VirtualDaysContent />
          </div>
        )}
      </main>
    </>
  )
}

/**
 * View sub-component - renders the main calendar content.
 */
function View() {
  const { settings } = useCalendarContext()
  const isScheduleMode = settings.viewMode === "schedule"

  return (
    <div className={`calendar-main-area${isScheduleMode ? " schedule-mode" : ""}`}>
      {/* Main days content (HourColumnFixed and AllDaySection rendered inside for proper scrolling) */}
      <Days />
    </div>
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
  View,
  TodayButton: TodayButtonWrapper,
  Fab,
  Modal,
})
