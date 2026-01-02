/**
 * ScheduleTimedEntry - renders a single timed entry in the schedule view.
 * Positioned absolutely based on start time and duration.
 * Supports sync status indicators for offline-first operations.
 */

import {
  calculateEntryWidth,
  calculateTimePositionPercent,
  type OverlapColumn,
} from "@core/calendar"
import type { ServerEntry } from "@core/calendar/types"
import { useCalendarContext } from "../contexts"
import { useEntrySyncStatus } from "../hooks/useEntrySyncStatus"
import { SyncIndicator } from "./SyncIndicator"

interface ScheduleTimedEntryProps {
  /** The entry to render */
  entry: ServerEntry
  /** Overlap column assignment */
  overlapColumn: OverlapColumn
  /** Width of the day column container */
  containerWidth: number
}

/**
 * Renders a timed entry tile in the schedule view.
 * Uses percentage-based absolute positioning for CSS-first layout.
 */
export function ScheduleTimedEntry({
  entry,
  overlapColumn,
  containerWidth,
}: ScheduleTimedEntryProps) {
  const { onEntryClick, flashStates, settings } = useCalendarContext()
  const syncStatus = useEntrySyncStatus(entry.id)
  const { entryStyle } = settings

  const { topPercent, heightPercent } = calculateTimePositionPercent(entry.startTime, entry.endTime)
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

  // Build CSS classes
  const classes = [
    "schedule-timed-entry",
    `entry-style-${entryStyle}`,
    flashClass,
    syncStatus === "pending" ? "schedule-timed-entry--pending" : "",
    syncStatus === "conflict" ? "schedule-timed-entry--conflict" : "",
  ]
    .filter(Boolean)
    .join(" ")

  return (
    // biome-ignore lint/a11y/useSemanticElements: Using div with role="button" for complex layout positioning
    <div
      className={classes}
      style={{
        top: `${topPercent}%`,
        height: `${heightPercent}%`,
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
      <SyncIndicator syncStatus={syncStatus} classPrefix="schedule-timed-entry" />
      <div className="schedule-timed-entry-time">{timeRange}</div>
      <div className="schedule-timed-entry-title">{entry.title}</div>
      {entry.location && <div className="schedule-timed-entry-location">{entry.location}</div>}
    </div>
  )
}
