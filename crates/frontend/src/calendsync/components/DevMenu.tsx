/**
 * DevMenu component for development tools.
 * Only renders when devMode is true (effectively excluded from production).
 */

import { useEffect, useRef, useState } from "react"
import type { InitialData } from "../types"

interface DevMenuProps {
  initialData: InitialData
  onToggleAnnotations?: () => void
  onClearAnnotations?: () => void
  onCopyAnnotations?: () => void
  annotationCount?: number
  isAnnotating?: boolean
}

/**
 * Dev menu with dropdown for development tools.
 * Shows a red "DEV" button that opens a dropdown with dev utilities.
 */
export function DevMenu({
  initialData,
  onToggleAnnotations,
  onClearAnnotations,
  onCopyAnnotations,
  annotationCount = 0,
  isAnnotating = false,
}: DevMenuProps) {
  const [isOpen, setIsOpen] = useState(false)
  const [copied, setCopied] = useState(false)
  const menuRef = useRef<HTMLDivElement>(null)

  // Close menu when clicking outside or pressing Escape
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setIsOpen(false)
      }
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setIsOpen(false)
      }
    }

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside)
      document.addEventListener("keydown", handleKeyDown)
      return () => {
        document.removeEventListener("mousedown", handleClickOutside)
        document.removeEventListener("keydown", handleKeyDown)
      }
    }
  }, [isOpen])

  // Only render in dev mode
  if (!initialData.devMode) {
    return null
  }

  const copyDesktopCommand = async () => {
    if (!initialData.sessionId) return

    const command = `CALENDSYNC_DEV_SESSION=${initialData.sessionId} cargo xtask dev desktop`
    try {
      await navigator.clipboard.writeText(command)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      // Clipboard API can fail due to permissions or non-secure context
      console.error("Failed to copy to clipboard")
    }
  }

  return (
    <div ref={menuRef} className="dev-menu">
      <button type="button" onClick={() => setIsOpen(!isOpen)} className="dev-menu-trigger">
        DEV
      </button>

      {/* Dropdown menu */}
      {isOpen && (
        <div className="dev-menu-dropdown">
          <div className="dev-menu-header">Dev Tools</div>

          {/* Annotation toggle */}
          <button
            type="button"
            onClick={() => {
              onToggleAnnotations?.()
              setIsOpen(false)
            }}
            className="dev-menu-item"
          >
            <div className="dev-menu-item-title">
              {isAnnotating ? "Stop Annotating" : "Annotate UI"}
            </div>
            <div className="dev-menu-item-subtitle">
              {annotationCount ? `${annotationCount} annotation(s)` : "Click elements to annotate"}
            </div>
          </button>

          {/* Clear and copy annotations when there are annotations */}
          {annotationCount > 0 && (
            <>
              <button
                type="button"
                onClick={() => {
                  onCopyAnnotations?.()
                  setIsOpen(false)
                }}
                className="dev-menu-item"
              >
                <div className="dev-menu-item-title">Copy Annotations</div>
                <div className="dev-menu-item-subtitle">Copy as markdown to clipboard</div>
              </button>
              <button
                type="button"
                onClick={() => {
                  onClearAnnotations?.()
                  setIsOpen(false)
                }}
                className="dev-menu-item"
              >
                <div className="dev-menu-item-title">Clear All Annotations</div>
                <div className="dev-menu-item-subtitle">
                  Remove all {annotationCount} annotation(s)
                </div>
              </button>
            </>
          )}

          {initialData.sessionId ? (
            <button type="button" onClick={copyDesktopCommand} className="dev-menu-item">
              <div className="dev-menu-item-title">
                {copied ? "Copied!" : "Copy Desktop Session Command"}
              </div>
              <div className="dev-menu-item-subtitle">
                Run desktop app with your current session
              </div>
            </button>
          ) : (
            <div className="dev-menu-empty">No session available</div>
          )}
        </div>
      )}
    </div>
  )
}
