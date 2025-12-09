/**
 * Focus trap hook for modal accessibility.
 * Imperative Shell: Handles DOM focus management with side effects.
 */

import { FOCUSABLE_SELECTOR, getNextFocusIndex } from "@core/calendar"
import { useCallback, useEffect, useRef } from "react"

/**
 * Configuration for useFocusTrap hook.
 */
export interface UseFocusTrapConfig {
  /** Whether the focus trap is currently active */
  isActive: boolean
  /** Callback when Escape key is pressed */
  onEscape?: () => void
  /** ID of the element to auto-focus on mount (optional) */
  autoFocusId?: string
}

/**
 * Result of useFocusTrap hook.
 */
export interface UseFocusTrapResult {
  /** Ref to attach to the container element */
  containerRef: React.RefObject<HTMLDivElement | null>
}

/**
 * Hook that creates a focus trap within a container element.
 *
 * Features:
 * - Traps Tab/Shift+Tab navigation within the container
 * - Auto-focuses the first focusable element (or specified element) on mount
 * - Restores focus to the previously focused element on unmount
 * - Handles Escape key with optional callback
 *
 * @example
 * const { containerRef } = useFocusTrap({
 *     isActive: true,
 *     onEscape: handleClose,
 *     autoFocusId: 'entry-title',
 * })
 *
 * return <div ref={containerRef}>...</div>
 */
export function useFocusTrap(config: UseFocusTrapConfig): UseFocusTrapResult {
  const { isActive, onEscape, autoFocusId } = config

  const containerRef = useRef<HTMLDivElement>(null)
  const previousActiveElement = useRef<HTMLElement | null>(null)

  /**
   * Get all focusable elements within the container.
   */
  const getFocusableElements = useCallback((): HTMLElement[] => {
    if (!containerRef.current) return []
    return Array.from(containerRef.current.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR))
  }, [])

  /**
   * Handle Tab key navigation.
   */
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (!isActive) return

      if (event.key === "Escape" && onEscape) {
        event.preventDefault()
        onEscape()
        return
      }

      if (event.key !== "Tab") return

      const focusableElements = getFocusableElements()
      if (focusableElements.length === 0) return

      const currentIndex = focusableElements.indexOf(document.activeElement as HTMLElement)
      const direction = event.shiftKey ? "backward" : "forward"
      const nextIndex = getNextFocusIndex(currentIndex, focusableElements.length, direction)

      // If we're about to leave the container, prevent default and wrap
      const isAtBoundary =
        (direction === "forward" && currentIndex === focusableElements.length - 1) ||
        (direction === "backward" && currentIndex === 0)

      if (isAtBoundary || currentIndex === -1) {
        event.preventDefault()
        focusableElements[nextIndex]?.focus()
      }
    },
    [isActive, onEscape, getFocusableElements],
  )

  /**
   * Store previously focused element and set up focus trap on mount.
   */
  useEffect(() => {
    if (!isActive) return

    // Store the currently focused element
    previousActiveElement.current = document.activeElement as HTMLElement

    // Auto-focus the specified element or first focusable element
    const focusTarget = autoFocusId
      ? document.getElementById(autoFocusId)
      : getFocusableElements()[0]

    if (focusTarget) {
      // Use setTimeout to ensure the modal is fully rendered
      setTimeout(() => focusTarget.focus(), 0)
    }

    // Add keydown listener
    document.addEventListener("keydown", handleKeyDown)

    return () => {
      document.removeEventListener("keydown", handleKeyDown)

      // Restore focus to the previously focused element
      if (previousActiveElement.current?.focus) {
        previousActiveElement.current.focus()
      }
    }
  }, [isActive, autoFocusId, getFocusableElements, handleKeyDown])

  return { containerRef }
}
