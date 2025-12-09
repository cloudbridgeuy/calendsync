/**
 * Today button component - floating button to jump to today.
 */

interface TodayButtonProps {
  /** Whether the button should be visible */
  visible: boolean
  /** Click handler */
  onClick: () => void
}

/**
 * Floating "Today" button that appears when not viewing today.
 */
export function TodayButton({ visible, onClick }: TodayButtonProps) {
  const classes = ["today-button", visible ? "visible" : ""].filter(Boolean).join(" ")

  return (
    <button type="button" className={classes} onClick={onClick}>
      Today
    </button>
  )
}
