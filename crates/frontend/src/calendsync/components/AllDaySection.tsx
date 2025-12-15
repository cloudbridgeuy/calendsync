/**
 * AllDaySection - renders the all-day entries section at the top of the schedule view.
 * Contains all-day events, multi-day events, and tasks in a horizontal band (Google Calendar style).
 */

import { formatDateKey, separateEntriesByType } from "@core/calendar"
import type { ServerEntry } from "@core/calendar/types"
import { useMemo } from "react"
import { useCalendarContext } from "../contexts"

/** Width of the hour column on the left */
const HOUR_COLUMN_WIDTH = 60

interface AllDayEntryTileProps {
  entry: ServerEntry
}

/**
 * Renders a single all-day/multi-day/task entry in the section.
 */
function AllDayEntryTile({ entry }: AllDayEntryTileProps) {
  const { onEntryClick, onEntryToggle, flashStates } = useCalendarContext()

  const flashState = flashStates.get(entry.id)
  const flashClass = flashState ? `flash-${flashState}` : ""

  // Determine badge text
  let badgeText = ""
  if (entry.isAllDay) {
    badgeText = "ALL DAY"
  } else if (entry.isMultiDay && entry.multiDayStart && entry.multiDayEnd) {
    badgeText = `${entry.multiDayStart} - ${entry.multiDayEnd}`
  }

  if (entry.isTask) {
    return (
      <div
        className={`all-day-entry task${entry.completed ? " completed" : ""}${flashClass ? ` ${flashClass}` : ""}`}
        style={{ borderLeftColor: entry.color || undefined }}
      >
        <label className="all-day-task-checkbox">
          <input type="checkbox" checked={entry.completed} onChange={() => onEntryToggle(entry)} />
          <span className="all-day-task-title">{entry.title}</span>
        </label>
      </div>
    )
  }

  return (
    // biome-ignore lint/a11y/useSemanticElements: Using div with role="button" for layout consistency with other entry tiles
    <div
      className={`all-day-entry${entry.isMultiDay ? " multi-day" : ""}${flashClass ? ` ${flashClass}` : ""}`}
      style={{ backgroundColor: entry.color || undefined }}
      onClick={() => onEntryClick(entry)}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          onEntryClick(entry)
        }
      }}
    >
      {badgeText && <span className="all-day-badge">{badgeText}</span>}
      <span className="all-day-title">{entry.title}</span>
    </div>
  )
}

interface AllDaySectionProps {
  /** Rendered dates in the virtual window */
  renderedDates: Date[]
  /** Width of each day column */
  dayWidth: number
  /** Get entries for a specific date */
  getEntriesForDate: (date: Date) => ServerEntry[]
}

/**
 * Renders the all-day section header with entries for each visible day.
 * Sticky at the top of the schedule view (Google Calendar style).
 */
export function AllDaySection({ renderedDates, dayWidth, getEntriesForDate }: AllDaySectionProps) {
  // Collect all-day entries for each date
  const allDayEntriesByDate = useMemo(() => {
    const map = new Map<string, ServerEntry[]>()
    for (const date of renderedDates) {
      const entries = getEntriesForDate(date)
      const { allDay, multiDay, tasks } = separateEntriesByType(entries)
      // Combine all-day, multi-day, and tasks for the top section
      map.set(formatDateKey(date), [...allDay, ...multiDay, ...tasks])
    }
    return map
  }, [renderedDates, getEntriesForDate])

  // Check if there are any all-day entries at all
  const hasAnyEntries = Array.from(allDayEntriesByDate.values()).some(
    (entries) => entries.length > 0,
  )

  if (!hasAnyEntries) {
    return null
  }

  return (
    <div className="all-day-section">
      {/* Spacer for hour column alignment */}
      <div
        className="all-day-spacer"
        style={{ width: HOUR_COLUMN_WIDTH, minWidth: HOUR_COLUMN_WIDTH }}
      />
      {/* Day columns for all-day entries */}
      <div className="all-day-columns">
        {renderedDates.map((date) => {
          const dateKey = formatDateKey(date)
          const entries = allDayEntriesByDate.get(dateKey) || []

          return (
            <div
              key={dateKey}
              className="all-day-column"
              style={{ width: dayWidth, minWidth: dayWidth }}
            >
              {entries.map((entry) => (
                <AllDayEntryTile key={entry.id} entry={entry} />
              ))}
            </div>
          )
        })}
      </div>
    </div>
  )
}
