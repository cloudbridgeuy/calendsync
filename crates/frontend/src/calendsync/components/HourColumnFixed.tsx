/**
 * HourColumnFixed - displays the hour labels on the left side of the schedule view.
 * Shows 24 hours from 12 AM to 11 PM.
 * Uses CSS sticky positioning to stay fixed during horizontal scroll.
 */

import { formatHourLabel, HOUR_HEIGHT_PX, HOURS_IN_DAY } from "@core/calendar"

/** Height of the day header row - must match .day-container-header height in CSS */
const DAY_HEADER_HEIGHT = 70

interface HourColumnFixedProps {
  /** Height of each hour row in pixels */
  hourHeight?: number
}

/**
 * Renders the hour labels column for the schedule view.
 * Fixed width, sticky left positioning.
 * Includes a spacer at the top to align with day headers.
 */
export function HourColumnFixed({ hourHeight = HOUR_HEIGHT_PX }: HourColumnFixedProps) {
  return (
    <div className="hour-column-fixed">
      {/* Spacer to align hour labels with time grid (below day headers) */}
      <div className="hour-column-header-spacer" style={{ height: DAY_HEADER_HEIGHT }} />
      {Array.from({ length: HOURS_IN_DAY }, (_, hour) => (
        <div key={`hour-${hour}`} className="hour-column-row" style={{ height: hourHeight }}>
          <span className="hour-column-label">{formatHourLabel(hour)}</span>
        </div>
      ))}
    </div>
  )
}
