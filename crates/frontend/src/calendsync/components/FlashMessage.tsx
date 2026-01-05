/**
 * Flash message component for server-to-client communication.
 *
 * Reads flash messages from cookies set by the server, displays them,
 * and clears the cookie after reading.
 */

import { useCallback, useEffect, useState } from "react"

export type FlashType = "error" | "success" | "info" | "warning"

export interface FlashMessageData {
  type: FlashType
  message: string
  autoDismiss: boolean
}

const AUTO_DISMISS_DURATION = 5000

/**
 * Parse a flash message from the cookie value.
 */
function parseFlashMessage(cookieValue: string): FlashMessageData | null {
  try {
    const decoded = decodeURIComponent(cookieValue)
    const parsed = JSON.parse(decoded)
    if (parsed && typeof parsed.type === "string" && typeof parsed.message === "string") {
      return {
        type: parsed.type as FlashType,
        message: parsed.message,
        autoDismiss: parsed.autoDismiss ?? false,
      }
    }
  } catch {
    // Invalid JSON or missing fields
  }
  return null
}

/**
 * Get the flash_message cookie value.
 */
function getFlashCookie(): string | null {
  if (typeof document === "undefined") return null

  const cookies = document.cookie.split("; ")
  for (const cookie of cookies) {
    const [name, ...valueParts] = cookie.split("=")
    if (name === "flash_message") {
      return valueParts.join("=")
    }
  }
  return null
}

/**
 * Clear the flash_message cookie.
 */
function clearFlashCookie(): void {
  if (typeof document === "undefined") return
  document.cookie = "flash_message=; Path=/; Max-Age=0"
}

interface FlashMessageProps {
  /** Optional callback when the flash message is dismissed */
  onDismiss?: () => void
}

export function FlashMessage({ onDismiss }: FlashMessageProps) {
  const [flash, setFlash] = useState<FlashMessageData | null>(null)
  const [isExiting, setIsExiting] = useState(false)

  const handleDismiss = useCallback(() => {
    setIsExiting(true)
    setTimeout(() => {
      setFlash(null)
      setIsExiting(false)
      onDismiss?.()
    }, 300)
  }, [onDismiss])

  // Read flash message from cookie on mount (client-side only)
  useEffect(() => {
    const cookieValue = getFlashCookie()
    if (cookieValue) {
      const parsed = parseFlashMessage(cookieValue)
      if (parsed) {
        setFlash(parsed)
        clearFlashCookie()
      }
    }
  }, [])

  // Auto-dismiss if configured
  useEffect(() => {
    if (!flash || !flash.autoDismiss) return

    const timer = setTimeout(() => {
      handleDismiss()
    }, AUTO_DISMISS_DURATION)

    return () => clearTimeout(timer)
  }, [flash, handleDismiss])

  if (!flash) return null

  const icons: Record<FlashType, string> = {
    error: "!",
    success: "\u2713",
    info: "i",
    warning: "!",
  }

  return (
    <div className={`flash-message flash-message-${flash.type}${isExiting ? " exiting" : ""}`}>
      <span className="flash-message-icon">{icons[flash.type]}</span>
      <span className="flash-message-text">{flash.message}</span>
      <button
        type="button"
        className="flash-message-close"
        onClick={handleDismiss}
        aria-label="Dismiss message"
      >
        &times;
      </button>
    </div>
  )
}
