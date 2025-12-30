/**
 * AllDayEntryTile - renders a single all-day/multi-day/task entry.
 * Used in the all-day section of schedule mode.
 */

import type { ServerEntry } from "@core/calendar/types"
import { useCalendarContext } from "../contexts"

interface AllDayEntryTileProps {
  entry: ServerEntry
}

/**
 * Renders a single all-day/multi-day/task entry tile.
 */
export function AllDayEntryTile({ entry }: AllDayEntryTileProps) {
  const { onEntryClick, onEntryToggle, flashStates, settings } = useCalendarContext()
  const { entryStyle } = settings

  const flashState = flashStates.get(entry.id)
  const flashClass = flashState ? `flash-${flashState}` : ""

  // Determine badge text
  let badgeText = ""
  if (entry.isAllDay) {
    badgeText = "ALL DAY"
  } else if (entry.isMultiDay && entry.startDate && entry.endDate) {
    badgeText = `${entry.startDate} - ${entry.endDate}`
  }

  // Apply color based on entry style setting
  const colorStyle = entry.color
    ? entryStyle === "filled"
      ? { backgroundColor: entry.color }
      : { borderLeftColor: entry.color }
    : undefined

  if (entry.isTask) {
    return (
      <div
        className={`all-day-entry entry-style-${entryStyle} task${entry.completed ? " completed" : ""}${flashClass ? ` ${flashClass}` : ""}`}
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
      className={`all-day-entry entry-style-${entryStyle}${entry.isMultiDay ? " multi-day" : ""}${flashClass ? ` ${flashClass}` : ""}`}
      style={colorStyle}
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
