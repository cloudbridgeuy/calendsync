/**
 * Calendar header component - shows month/year and week bar.
 */

import { addDays, isSameDay } from "@core/calendar/dates"
import { DAY_NAMES, DAY_NAMES_FULL, MONTH_NAMES } from "@core/calendar/types"

interface CalendarHeaderProps {
  /** The current center/highlighted date */
  centerDate: Date
  /** Number of visible days */
  visibleDays: number
  /** Whether the viewport is mobile */
  isMobile: boolean
  /** Navigate to a specific day offset from center */
  onNavigate: (offset: number) => void
}

/**
 * Get today's date at midnight.
 */
function getToday(): Date {
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  return today
}

/**
 * Calendar header with month/year display and week bar (desktop).
 */
export function CalendarHeader({
  centerDate,
  visibleDays,
  isMobile,
  onNavigate,
}: CalendarHeaderProps) {
  const month = MONTH_NAMES[centerDate.getMonth()]
  const year = centerDate.getFullYear()
  const today = getToday()

  // Mobile: Single day display
  const mobileDayDisplay = isMobile ? (
    <div className="day-display">
      <div className="day-number">{centerDate.getDate()}</div>
      <div className="day-name">{DAY_NAMES_FULL[centerDate.getDay()]}</div>
    </div>
  ) : null

  // Desktop: Week bar
  const halfDays = Math.floor(visibleDays / 2)
  const weekBar = !isMobile ? (
    <nav className="week-bar">
      {Array.from({ length: visibleDays }).map((_, i) => {
        const date = addDays(centerDate, i - halfDays)
        const isCenter = i === halfDays
        const isToday = isSameDay(date, today)
        const offset = i - halfDays

        const classes = ["week-day", isCenter ? "highlighted" : "", isToday ? "is-today" : ""]
          .filter(Boolean)
          .join(" ")

        return (
          <button
            key={offset}
            type="button"
            className={classes}
            style={{ flex: 1 }}
            onClick={() => onNavigate(offset)}
          >
            <div className="week-day-number">{date.getDate()}</div>
            <div className="week-day-name">{DAY_NAMES[date.getDay()]}</div>
          </button>
        )
      })}
    </nav>
  ) : null

  return (
    <header className="calendar-header">
      <div className="month-year">
        {month} {year}
      </div>
      {mobileDayDisplay}
      {weekBar}
    </header>
  )
}
