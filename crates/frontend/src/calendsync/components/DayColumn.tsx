/**
 * Day column component - displays entries for a single day.
 * Renders differently based on viewMode: compact (list) or schedule (24-hour grid).
 */

import {
  calculateGridHeight,
  detectOverlappingEntries,
  HOUR_HEIGHT_PX,
  HOURS_IN_DAY,
  separateEntriesByType,
  sortDayEntries,
} from "@core/calendar"
import type { ViewMode } from "@core/calendar/settings"
import type { ServerEntry } from "@core/calendar/types"
import { useMemo } from "react"

import { EntryTile } from "./EntryTile"
import { ScheduleTimedEntry } from "./ScheduleTimedEntry"

interface DayColumnProps {
  /** Date key (YYYY-MM-DD) */
  dateKey: string
  /** Entries for this day */
  entries: ServerEntry[]
  /** CSS style for positioning */
  style?: React.CSSProperties
  /** Whether this is the last visible column */
  isLastVisible?: boolean
  /** View mode - compact (list) or schedule (24-hour grid) */
  viewMode?: ViewMode
  /** Width of day column (needed for schedule mode entry width calculation) */
  dayWidth?: number
}

/**
 * Render an empty day column.
 */
export function EmptyDayColumn() {
  return (
    <div className="empty-day">
      <div className="empty-day-icon">📅</div>
      <div className="empty-day-text">No events</div>
    </div>
  )
}

/**
 * Render a single day column with its entries.
 * In compact mode: renders a list of entry tiles.
 * In schedule mode: renders a 24-hour grid with positioned entries.
 */
export function DayColumn({
  dateKey,
  entries,
  style,
  isLastVisible,
  viewMode = "compact",
  dayWidth = 0,
}: DayColumnProps) {
  if (viewMode === "schedule") {
    return <ScheduleDayContent entries={entries} dayWidth={dayWidth} dateKey={dateKey} />
  }

  // Compact mode: sort and render as list
  const sortedEntries = sortDayEntries(entries)
  const classes = ["day-column", isLastVisible ? "last-visible" : ""].filter(Boolean).join(" ")

  return (
    <div className={classes} data-date={dateKey} style={style}>
      <EntryTiles entries={sortedEntries} />
    </div>
  )
}

interface EntryTilesProps {
  /** Entries for this day */
  entries: ServerEntry[]
}

/**
 * Render the DayColumn entries tiles (compact mode).
 */
export function EntryTiles({ entries }: EntryTilesProps) {
  if (entries.length === 0) {
    return <EmptyDayColumn />
  }

  return (
    <>
      {entries.map((entry) => (
        <EntryTile key={entry.id} entry={entry} />
      ))}
    </>
  )
}

interface ScheduleDayContentProps {
  /** Entries for this day */
  entries: ServerEntry[]
  /** Width of day column */
  dayWidth: number
  /** Date key for data attribute */
  dateKey: string
}

/**
 * Render the schedule view content for a single day.
 * Shows hour grid lines and absolutely positioned timed entries.
 * All-day, multi-day, and tasks are rendered in the AllDaySection component.
 */
function ScheduleDayContent({ entries, dayWidth, dateKey }: ScheduleDayContentProps) {
  // Separate timed entries from all-day/multi-day/tasks
  const { timed } = useMemo(() => separateEntriesByType(entries), [entries])

  // Calculate overlap columns for timed entries
  const overlapColumns = useMemo(() => detectOverlappingEntries(timed), [timed])

  const gridHeight = calculateGridHeight()

  return (
    <div className="schedule-day-content" data-date={dateKey} style={{ height: gridHeight }}>
      {/* Hour grid lines */}
      {Array.from({ length: HOURS_IN_DAY }, (_, hour) => (
        <div
          key={`line-${hour}`}
          className="schedule-hour-line"
          style={{ top: hour * HOUR_HEIGHT_PX }}
        />
      ))}

      {/* Timed entries */}
      {timed.map((entry) => {
        const overlapColumn = overlapColumns.get(entry.id)
        if (!overlapColumn) return null

        return (
          <ScheduleTimedEntry
            key={entry.id}
            entry={entry}
            overlapColumn={overlapColumn}
            containerWidth={dayWidth}
          />
        )
      })}
    </div>
  )
}
