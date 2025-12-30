/**
 * ScheduleTimedEntry - renders a single timed entry in the schedule view.
 * Positioned absolutely based on start time and duration.
 */

import {
  calculateEntryWidth,
  calculateTimePosition,
  HOUR_HEIGHT_PX,
  type OverlapColumn,
} from "@core/calendar"
import type { ServerEntry } from "@core/calendar/types"
import { useCalendarContext } from "../contexts"

interface ScheduleTimedEntryProps {
  /** The entry to render */
  entry: ServerEntry
  /** Overlap column assignment */
  overlapColumn: OverlapColumn
  /** Width of the day column container */
  containerWidth: number
  /** Height of each hour in pixels */
  hourHeight?: number
}

/**
 * Renders a timed entry tile in the schedule view.
 * Uses absolute positioning based on time and overlap columns.
 */
export function ScheduleTimedEntry({
  entry,
  overlapColumn,
  containerWidth,
  hourHeight = HOUR_HEIGHT_PX,
}: ScheduleTimedEntryProps) {
  const { onEntryClick, flashStates, settings } = useCalendarContext()
  const { entryStyle } = settings

  const { top, height } = calculateTimePosition(entry.startTime, entry.endTime, hourHeight)
  const { width, left } = calculateEntryWidth(overlapColumn, containerWidth)

  const flashState = flashStates.get(entry.id)
  const flashClass = flashState ? `flash-${flashState}` : ""

  // Format time range for display
  const timeRange =
    entry.startTime && entry.endTime
      ? `${entry.startTime.slice(0, 5)} - ${entry.endTime.slice(0, 5)}`
      : ""

  // Apply color based on entry style setting
  const colorStyle = entry.color
    ? entryStyle === "filled"
      ? { backgroundColor: entry.color }
      : { borderLeftColor: entry.color }
    : undefined

  return (
    // biome-ignore lint/a11y/useSemanticElements: Using div with role="button" for complex layout positioning
    <div
      className={`schedule-timed-entry entry-style-${entryStyle}${flashClass ? ` ${flashClass}` : ""}`}
      style={{
        top,
        height,
        left,
        width,
        ...colorStyle,
      }}
      onClick={() => onEntryClick(entry)}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          onEntryClick(entry)
        }
      }}
    >
      <div className="schedule-timed-entry-time">{timeRange}</div>
      <div className="schedule-timed-entry-title">{entry.title}</div>
      {entry.location && <div className="schedule-timed-entry-location">{entry.location}</div>}
    </div>
  )
}
