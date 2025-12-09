/**
 * Calendar header component - shows month/year display.
 */

import { MONTH_NAMES } from "@core/calendar/types"

interface CalendarHeaderProps {
  /** The currently highlighted date (for month/year display) */
  highlightedDate: Date
}

/**
 * Calendar header with month/year display.
 */
export function CalendarHeader({ highlightedDate }: CalendarHeaderProps) {
  const month = MONTH_NAMES[highlightedDate.getMonth()]
  const year = highlightedDate.getFullYear()

  return (
    <header className="calendar-header">
      <div className="month-year">
        {month} {year}
      </div>
    </header>
  )
}
