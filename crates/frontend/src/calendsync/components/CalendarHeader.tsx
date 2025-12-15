/**
 * Calendar header component - shows month/year display and settings menu.
 */

import { MONTH_NAMES } from "@core/calendar/types"

interface CalendarHeaderProps {
  /** The currently highlighted date (for month/year display) */
  highlightedDate: Date
  /** Optional settings menu slot (rendered on the left) */
  settingsSlot?: React.ReactNode
}

/**
 * Calendar header with month/year display and optional settings menu.
 */
export function CalendarHeader({ highlightedDate, settingsSlot }: CalendarHeaderProps) {
  const month = MONTH_NAMES[highlightedDate.getMonth()]
  const year = highlightedDate.getFullYear()

  return (
    <header className="calendar-header">
      {settingsSlot && <div className="calendar-header-left">{settingsSlot}</div>}
      <div className="month-year">
        {month} {year}
      </div>
    </header>
  )
}
