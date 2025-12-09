/**
 * DayContainer compound component - displays day header with sticky positioning and scrollable content.
 * Uses context to share date and highlight state between Header and Content sub-components.
 */

import { getDayDisplayInfo } from "@core/calendar"
import { createContext, useContext, useMemo } from "react"

/** Context value shared with DayContainer sub-components */
export interface DayContainerContextValue {
  /** The date for this day container */
  date: Date
  /** Whether this day is highlighted */
  isHighlighted: boolean
  /** Callback when header is clicked */
  onHeaderClick?: () => void
}

/** DayContainerContext - null when not inside provider */
const DayContainerContext = createContext<DayContainerContextValue | null>(null)

/**
 * Hook to access DayContainerContext.
 * Throws if used outside DayContainerRoot.
 */
function useDayContainerContext(): DayContainerContextValue {
  const ctx = useContext(DayContainerContext)
  if (!ctx) {
    throw new Error("DayContainer sub-components must be used within DayContainerRoot")
  }
  return ctx
}

// ============================================================================
// Header Sub-Component
// ============================================================================

/**
 * DayContainer.Header - sticky day header showing number, day name, and today badge.
 * Gets date and highlight state from context. Clicking navigates to this day.
 */
function Header() {
  const { date, isHighlighted, onHeaderClick } = useDayContainerContext()
  const displayInfo = getDayDisplayInfo(date)

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault()
      onHeaderClick?.()
    }
  }

  return (
    <div
      className={`day-container-header${isHighlighted ? " highlighted" : ""}`}
      onClick={onHeaderClick}
      onKeyDown={handleKeyDown}
      role="button"
      tabIndex={0}
      aria-label={`Navigate to ${displayInfo.dayName} ${displayInfo.dayNumber}`}
    >
      <div className="day-container-number">{displayInfo.dayNumber}</div>
      <div className="day-container-name">{displayInfo.dayName}</div>
      {displayInfo.isToday && <div className="day-container-today">TODAY</div>}
    </div>
  )
}

// ============================================================================
// Content Sub-Component
// ============================================================================

interface ContentProps {
  children: React.ReactNode
}

/**
 * DayContainer.Content - scrollable content area for day entries.
 * This is where DayColumn will be placed.
 */
function Content({ children }: ContentProps) {
  return <div className="day-container-content">{children}</div>
}

// ============================================================================
// Main Component + Compound Export
// ============================================================================

interface DayContainerProps {
  /** The date for this day container */
  date: Date
  /** Width of the day container in pixels */
  dayWidth: number
  /** Whether this day is highlighted */
  isHighlighted: boolean
  /** Callback when header is clicked */
  onHeaderClick?: () => void
  /** Child components (Header and Content) */
  children: React.ReactNode
}

/**
 * DayContainer compound component - main container with sticky header and scrollable content.
 *
 * @example
 * <DayContainer date={date} dayWidth={120} isHighlighted={false}>
 *   <DayContainer.Header />
 *   <DayContainer.Content>
 *     <DayColumn dateKey={dateKey} entries={entries} />
 *   </DayContainer.Content>
 * </DayContainer>
 */
function DayContainerRoot({
  date,
  dayWidth,
  isHighlighted,
  onHeaderClick,
  children,
}: DayContainerProps) {
  const contextValue = useMemo<DayContainerContextValue>(
    () => ({
      date,
      isHighlighted,
      onHeaderClick,
    }),
    [date, isHighlighted, onHeaderClick],
  )

  return (
    <DayContainerContext.Provider value={contextValue}>
      <div className="day-container" style={{ width: dayWidth, minWidth: dayWidth }}>
        {children}
      </div>
    </DayContainerContext.Provider>
  )
}

// Attach sub-components as static properties
export const DayContainer = Object.assign(DayContainerRoot, {
  Header,
  Content,
})
