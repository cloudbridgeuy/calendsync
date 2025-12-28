/**
 * AllDayToggle - clickable toggle button for showing/hiding entries in the all-day section.
 * Used for both the overflow toggle "(+N more)" and the tasks toggle "(N tasks)".
 */

interface AllDayToggleProps {
  /** Toggle text to display */
  text: string
  /** Click handler to toggle visibility */
  onClick: () => void
}

/**
 * Render a toggle button for the all-day section.
 * Styled as a subtle, clickable text that changes appearance on hover.
 */
export function AllDayToggle({ text, onClick }: AllDayToggleProps) {
  return (
    <button type="button" className="all-day-toggle" onClick={onClick}>
      {text}
    </button>
  )
}
