/**
 * Day column component - displays entries for a single day.
 */

import { sortDayEntries } from "@core/calendar/entries"
import type { ServerEntry } from "@core/calendar/types"

import { EntryTile } from "./EntryTile"

interface DayColumnProps {
  /** Date key (YYYY-MM-DD) */
  dateKey: string
  /** Entries for this day */
  entries: ServerEntry[]
  /** CSS style for positioning */
  style?: React.CSSProperties
  /** Whether this is the last visible column */
  isLastVisible?: boolean
}

/**
 * Render a single day column with its entries.
 */
export function DayColumn({ dateKey, entries, style, isLastVisible }: DayColumnProps) {
  // Sort entries: all-day first, then multi-day, then timed by start time, then tasks
  const sortedEntries = sortDayEntries(entries)

  const classes = ["day-column", isLastVisible ? "last-visible" : ""].filter(Boolean).join(" ")

  // Empty state - no entries for this day
  if (sortedEntries.length === 0) {
    return (
      <div className={classes} data-date={dateKey} style={style}>
        <div className="empty-day">
          <div className="empty-day-icon">📅</div>
          <div className="empty-day-text">No events</div>
        </div>
      </div>
    )
  }

  return (
    <div className={classes} data-date={dateKey} style={style}>
      {sortedEntries.map((entry) => (
        <EntryTile key={entry.id} entry={entry} />
      ))}
    </div>
  )
}
