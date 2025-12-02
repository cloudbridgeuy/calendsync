/**
 * Entry tile component - displays a single calendar entry.
 */

import type { ServerEntry } from "@core/calendar/types"
import type { FlashState } from "../types"

interface EntryTileProps {
    entry: ServerEntry
    flashState?: FlashState
}

/**
 * Render a single entry tile.
 */
export function EntryTile({ entry, flashState }: EntryTileProps) {
    const colorStyle = entry.color ? { borderLeftColor: entry.color } : undefined

    // Build CSS classes
    const classes = [
        "entry-tile",
        entry.kind,
        entry.completed ? "completed" : "",
        flashState ? `flash-${flashState}` : "",
    ]
        .filter(Boolean)
        .join(" ")

    // Badge for multi-day or all-day events
    let badge: React.ReactNode = null
    if (entry.isMultiDay && entry.multiDayStart && entry.multiDayEnd) {
        badge = (
            <div className="entry-badge">
                {entry.multiDayStart} - {entry.multiDayEnd}
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

    // Task entries have a checkbox layout
    if (entry.isTask) {
        return (
            <div className={classes} style={colorStyle} data-entry-id={entry.id}>
                <div className="task-row">
                    <div className={`task-checkbox${entry.completed ? " checked" : ""}`} />
                    <div>{content}</div>
                </div>
            </div>
        )
    }

    // Regular entry layout
    return (
        <div className={classes} style={colorStyle} data-entry-id={entry.id}>
            {badge}
            {time}
            {content}
        </div>
    )
}
