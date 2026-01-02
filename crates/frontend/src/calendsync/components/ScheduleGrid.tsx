/**
 * ScheduleGrid - compound component for schedule mode layout.
 * Uses CSS Grid for proper alignment of hour labels, day headers, and content.
 *
 * Layout structure:
 * +------------------+---------------------------+
 * | Corner cell      | Day headers (sticky top)  |  Row 1
 * +------------------+---------------------------+
 * | "all-day" label  | All-day events            |  Row 2 (sticky below row 1)
 * +------------------+---------------------------+
 * | Hour column      | Timed content grid        |  Row 3 (scrollable)
 * | (sticky left)    |                           |
 * +------------------+---------------------------+
 */

import {
  computeAllDaySummary,
  filterAllDayEntries,
  filterTimedEntries,
  formatDateKey,
  formatHourLabel,
  formatOverflowToggle,
  formatTasksToggle,
  getDayDisplayInfo,
  getTimezoneAbbreviation,
  HOURS_IN_DAY,
  isSameCalendarDay,
} from "@core/calendar"
import type { ServerEntry } from "@core/calendar/types"
import { createContext, useContext, useMemo } from "react"
import { useCalendarContext } from "../contexts"
import { AllDayEntryTile } from "./AllDayEntryTile"
import { AllDayToggle } from "./AllDayToggle"
import { ScheduleDayContent } from "./DayColumn"

// ============================================================================
// Context
// ============================================================================

interface ScheduleGridContextValue {
  renderedDates: Date[]
  getEntriesForDate: (date: Date) => ServerEntry[]
  dayWidth: number
  highlightedDate: Date
  scrollToDate: (date: Date) => void
}

const ScheduleGridContext = createContext<ScheduleGridContextValue | null>(null)

function useScheduleGridContext(): ScheduleGridContextValue {
  const ctx = useContext(ScheduleGridContext)
  if (!ctx) {
    throw new Error("ScheduleGrid sub-components must be used within ScheduleGrid")
  }
  return ctx
}

// ============================================================================
// Main Container
// ============================================================================

interface ScheduleGridRootProps {
  children: React.ReactNode
}

function ScheduleGridRoot({ children }: ScheduleGridRootProps) {
  const { renderedDates, getEntriesForDate, dayWidth, highlightedDate, scrollToDate } =
    useCalendarContext()

  const contextValue = useMemo<ScheduleGridContextValue>(
    () => ({
      renderedDates,
      getEntriesForDate,
      dayWidth,
      highlightedDate,
      scrollToDate,
    }),
    [renderedDates, getEntriesForDate, dayWidth, highlightedDate, scrollToDate],
  )

  return (
    <ScheduleGridContext.Provider value={contextValue}>
      <div className="schedule-grid">{children}</div>
    </ScheduleGridContext.Provider>
  )
}

// ============================================================================
// Sub-Components
// ============================================================================

/**
 * Corner cell - top-left corner of the grid.
 * Sticky at top and left. Displays the current timezone abbreviation.
 */
function Corner() {
  const timezone = useMemo(() => getTimezoneAbbreviation(new Date()), [])

  return (
    <div className="schedule-corner">
      <span className="schedule-timezone">{timezone}</span>
    </div>
  )
}

/**
 * Day headers row - sticky at top.
 */
function DayHeaders() {
  const { renderedDates, dayWidth, highlightedDate, scrollToDate } = useScheduleGridContext()

  return (
    <div className="schedule-day-headers">
      {renderedDates.map((date) => {
        const dateKey = formatDateKey(date)
        const isHighlighted = isSameCalendarDay(date, highlightedDate)
        const displayInfo = getDayDisplayInfo(date)

        return (
          // biome-ignore lint/a11y/useSemanticElements: Using div with role="button" to preserve existing styling
          <div
            key={dateKey}
            className={`schedule-day-header${isHighlighted ? " highlighted" : ""}`}
            style={{ width: dayWidth, minWidth: dayWidth }}
            onClick={() => scrollToDate(date)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                scrollToDate(date)
              }
            }}
            role="button"
            tabIndex={0}
          >
            <div className="schedule-day-number">{displayInfo.dayNumber}</div>
            <div className="schedule-day-name">{displayInfo.dayName}</div>
            {displayInfo.isToday && <div className="schedule-day-today">TODAY</div>}
          </div>
        )
      })}
    </div>
  )
}

/**
 * All-day label - left side label for all-day section.
 * Sticky at top (below headers) and left.
 */
function AllDayLabel() {
  return <div className="schedule-all-day-label">all-day</div>
}

/**
 * All-day events section - shows all-day, multi-day, and task entries.
 * Sticky below day headers. Supports collapsible overflow and task toggles.
 */
function AllDayEvents() {
  const { renderedDates, getEntriesForDate, dayWidth } = useScheduleGridContext()
  const { showAllDayOverflow, setShowAllDayOverflow, showAllDayTasks, setShowAllDayTasks } =
    useCalendarContext()

  // Compute all-day summaries for each date
  const summariesByDate = useMemo(() => {
    const map = new Map<string, ReturnType<typeof computeAllDaySummary>>()
    for (const date of renderedDates) {
      const entries = filterAllDayEntries(getEntriesForDate(date))
      const summary = computeAllDaySummary(entries, showAllDayOverflow)
      map.set(formatDateKey(date), summary)
    }
    return map
  }, [renderedDates, getEntriesForDate, showAllDayOverflow])

  // Check if there are any entries across all dates
  const hasAnyEntries = Array.from(summariesByDate.values()).some(
    (summary) => summary.visibleEvents.length > 0 || summary.tasks.length > 0,
  )

  // Still render the container for layout consistency, but empty if no entries
  return (
    <div className={`schedule-all-day-events${hasAnyEntries ? "" : " empty"}`}>
      {renderedDates.map((date) => {
        const dateKey = formatDateKey(date)
        const summary = summariesByDate.get(dateKey)
        if (!summary) return null

        // Compute per-column toggle text
        const columnOverflowText = formatOverflowToggle(
          summary.hiddenEventCount,
          showAllDayOverflow,
        )
        const columnTasksText = formatTasksToggle(summary.tasks.length, showAllDayTasks)

        return (
          <div
            key={dateKey}
            className="schedule-all-day-column"
            style={{ width: dayWidth, minWidth: dayWidth }}
          >
            {/* Visible events */}
            {summary.visibleEvents.map((entry) => (
              <AllDayEntryTile key={entry.id} entry={entry} />
            ))}

            {/* Overflow toggle */}
            {columnOverflowText && (
              <AllDayToggle
                text={columnOverflowText}
                onClick={() => setShowAllDayOverflow(!showAllDayOverflow)}
              />
            )}

            {/* Tasks (when expanded) */}
            {showAllDayTasks &&
              summary.tasks.map((entry) => <AllDayEntryTile key={entry.id} entry={entry} />)}

            {/* Tasks toggle */}
            {columnTasksText && (
              <AllDayToggle
                text={columnTasksText}
                onClick={() => setShowAllDayTasks(!showAllDayTasks)}
              />
            )}
          </div>
        )
      })}
    </div>
  )
}

/**
 * Hour column - left side hour labels.
 * Sticky at left edge.
 * Uses CSS flexbox for height - rows expand proportionally with the grid.
 */
function HourColumn() {
  return (
    <div className="schedule-hour-column">
      {Array.from({ length: HOURS_IN_DAY }, (_, hour) => (
        <div key={`hour-${hour}`} className="schedule-hour-row">
          <span className="schedule-hour-label">{formatHourLabel(hour)}</span>
        </div>
      ))}
    </div>
  )
}

/**
 * Timed grid - main content area with timed entries.
 */
function TimedGrid() {
  const { renderedDates, getEntriesForDate, dayWidth } = useScheduleGridContext()

  return (
    <div className="schedule-timed-grid">
      {renderedDates.map((date) => {
        const dateKey = formatDateKey(date)
        const entries = filterTimedEntries(getEntriesForDate(date))

        return (
          <ScheduleDayContent
            key={dateKey}
            entries={entries}
            dayWidth={dayWidth}
            dateKey={dateKey}
          />
        )
      })}
    </div>
  )
}

// ============================================================================
// Export Compound Component
// ============================================================================

export const ScheduleGrid = Object.assign(ScheduleGridRoot, {
  Corner,
  DayHeaders,
  AllDayLabel,
  AllDayEvents,
  HourColumn,
  TimedGrid,
})
