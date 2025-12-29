/**
 * Entry tile component - displays a single calendar entry.
 * Consumes CalendarContext for flash states and callbacks.
 */

import type { ServerEntry } from "@core/calendar/types"
import { useCalendarContext } from "../contexts"

interface EntryTileProps {
  entry: ServerEntry
}

/**
 * Render a single entry tile.
 */
export function EntryTile({ entry }: EntryTileProps) {
  const { flashStates, onEntryClick, onEntryToggle } = useCalendarContext()
  const flashState = flashStates.get(entry.id)

  const colorStyle = entry.color ? { borderLeftColor: entry.color } : undefined

  // Build CSS classes
  const classes = [
    "entry-tile",
    entry.kind,
    entry.completed ? "completed" : "",
    flashState ? `flash-${flashState}` : "",
    "clickable",
  ]
    .filter(Boolean)
    .join(" ")

  // Handle click to open edit modal
  const handleClick = () => {
    onEntryClick(entry)
  }

  // Handle keyboard activation (Enter/Space)
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault()
      handleClick()
    }
  }

  // Common interactive props
  const interactiveProps = {
    onClick: handleClick,
    onKeyDown: handleKeyDown,
    role: "button" as const,
    tabIndex: 0,
  }

  // Badge for multi-day or all-day events
  let badge: React.ReactNode = null
  if (entry.isMultiDay && entry.startDate && entry.endDate) {
    badge = (
      <div className="entry-badge">
        {entry.startDate} - {entry.endDate}
      </div>
    )
  } else if (entry.isAllDay) {
    badge = <div className="entry-badge">All Day</div>
  }

  // Time display for timed events
  let time: React.ReactNode = null
  if (entry.isTimed && entry.startTime && entry.endTime) {
    time = (
      <div className="entry-time">
        {entry.startTime} - {entry.endTime}
      </div>
    )
  }

  // Entry content
  const content = (
    <>
      <div className="entry-title">{entry.title}</div>
      {entry.description && <div className="entry-description">{entry.description}</div>}
      {entry.location && <div className="entry-location">{entry.location}</div>}
    </>
  )

  // Handle checkbox click for tasks
  const handleCheckboxClick = (e: React.MouseEvent) => {
    e.stopPropagation() // Prevent opening the modal
    onEntryToggle(entry)
  }

  // Handle checkbox keyboard activation
  const handleCheckboxKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault()
      e.stopPropagation()
      onEntryToggle(entry)
    }
  }

  // Task entries have a checkbox layout
  if (entry.isTask) {
    return (
      <div className={classes} style={colorStyle} data-entry-id={entry.id} {...interactiveProps}>
        <div className="task-row">
          <input
            type="checkbox"
            className="task-checkbox"
            checked={entry.completed}
            onClick={handleCheckboxClick}
            onKeyDown={handleCheckboxKeyDown}
            onChange={() => {}} // Controlled by onClick
            aria-label={`Mark ${entry.title} as ${entry.completed ? "incomplete" : "complete"}`}
          />
          <div>{content}</div>
        </div>
      </div>
    )
  }

  // Regular entry layout
  return (
    <div className={classes} style={colorStyle} data-entry-id={entry.id} {...interactiveProps}>
      {badge}
      {time}
      {content}
    </div>
  )
}
