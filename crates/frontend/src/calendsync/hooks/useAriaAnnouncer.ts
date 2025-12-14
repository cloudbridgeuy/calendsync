/**
 * useAriaAnnouncer - Hook for managing ARIA live region announcements.
 *
 * Creates an invisible live region for screen reader announcements.
 * The region is automatically cleaned up on unmount.
 */

import { useCallback, useEffect, useRef } from "react"

export interface UseAriaAnnouncerReturn {
  /** Announce a message to screen readers */
  announce: (message: string, priority?: "polite" | "assertive") => void
}

/**
 * Hook for screen reader announcements via ARIA live region.
 */
export function useAriaAnnouncer(): UseAriaAnnouncerReturn {
  const regionRef = useRef<HTMLDivElement | null>(null)

  // Create live region on mount
  useEffect(() => {
    const region = document.createElement("div")
    region.setAttribute("role", "status")
    region.setAttribute("aria-live", "polite")
    region.setAttribute("aria-atomic", "true")
    region.className = "sr-only"
    document.body.appendChild(region)
    regionRef.current = region

    return () => {
      if (regionRef.current) {
        document.body.removeChild(regionRef.current)
      }
    }
  }, [])

  const announce = useCallback((message: string, priority: "polite" | "assertive" = "polite") => {
    if (!regionRef.current) return

    regionRef.current.setAttribute("aria-live", priority)
    // Clear and set to trigger announcement
    regionRef.current.textContent = ""
    // Use setTimeout to ensure the clear is processed first
    setTimeout(() => {
      if (regionRef.current) {
        regionRef.current.textContent = message
      }
    }, 50)
  }, [])

  return { announce }
}
