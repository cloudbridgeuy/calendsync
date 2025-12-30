/**
 * SettingsMenu compound component - displays settings gear icon with dropdown panel.
 * Uses context to share state between Trigger, Panel, and toggle sub-components.
 */

import type { EntryStyle, ViewMode } from "@core/calendar/settings"
import { useCallback, useEffect } from "react"
import { SettingsMenuProvider, useSettingsMenuContext } from "../contexts/SettingsMenuContext"

// ============================================================================
// Trigger Sub-Component
// ============================================================================

/**
 * SettingsMenu.Trigger - the gear icon button.
 */
function Trigger() {
  const { state, actions, triggerId, contentId, buttonRef } = useSettingsMenuContext()
  const { isOpen } = state
  const { toggleOpen } = actions

  return (
    <button
      ref={buttonRef}
      id={triggerId}
      type="button"
      className={`settings-trigger${isOpen ? " active" : ""}`}
      onClick={toggleOpen}
      aria-label="Calendar settings"
      aria-expanded={isOpen}
      aria-controls={contentId}
      aria-haspopup="dialog"
    >
      <svg
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
    </button>
  )
}

// ============================================================================
// Panel Sub-Component
// ============================================================================

interface PanelProps {
  children: React.ReactNode
}

/**
 * SettingsMenu.Panel - the dropdown panel containing settings options.
 */
function Panel({ children }: PanelProps) {
  const { state, actions, triggerId, contentId, panelRef } = useSettingsMenuContext()
  const { isOpen } = state
  const { close } = actions

  // Close on Escape key
  useEffect(() => {
    if (!isOpen) return

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        close()
      }
    }

    document.addEventListener("keydown", handleEscape)
    return () => document.removeEventListener("keydown", handleEscape)
  }, [isOpen, close])

  // Close on click outside
  useEffect(() => {
    if (!isOpen) return

    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as Node
      const panel = document.getElementById(contentId)
      const button = document.getElementById(triggerId)

      if (panel && !panel.contains(target) && button && !button.contains(target)) {
        close()
      }
    }

    document.addEventListener("mousedown", handleClickOutside)
    return () => document.removeEventListener("mousedown", handleClickOutside)
  }, [isOpen, close, contentId, triggerId])

  if (!isOpen) return null

  return (
    <>
      {/* Backdrop - purely decorative overlay */}
      <div className="settings-backdrop" onClick={close} aria-hidden="true" />

      <div
        ref={panelRef}
        id={contentId}
        className="settings-panel"
        role="dialog"
        aria-labelledby={triggerId}
        aria-label="Calendar settings"
      >
        <div className="settings-panel-header">
          <h2 className="settings-panel-title">Settings</h2>
        </div>
        <div className="settings-panel-content">{children}</div>
      </div>
    </>
  )
}

// ============================================================================
// ViewToggle Sub-Component
// ============================================================================

/**
 * SettingsMenu.ViewToggle - radio buttons for Compact / Schedule view modes.
 */
function ViewToggle() {
  const { state, actions } = useSettingsMenuContext()
  const { viewMode } = state
  const { setViewMode } = actions

  const handleChange = useCallback(
    (mode: ViewMode) => {
      setViewMode(mode)
    },
    [setViewMode],
  )

  return (
    <div className="settings-option">
      <span className="settings-option-label">View</span>
      <div className="settings-radio-group" role="radiogroup" aria-label="View mode">
        <label className={`settings-radio${viewMode === "compact" ? " selected" : ""}`}>
          <input
            type="radio"
            name="viewMode"
            value="compact"
            checked={viewMode === "compact"}
            onChange={() => handleChange("compact")}
          />
          <span className="settings-radio-text">Compact</span>
        </label>
        <label className={`settings-radio${viewMode === "schedule" ? " selected" : ""}`}>
          <input
            type="radio"
            name="viewMode"
            value="schedule"
            checked={viewMode === "schedule"}
            onChange={() => handleChange("schedule")}
          />
          <span className="settings-radio-text">Schedule</span>
        </label>
      </div>
    </div>
  )
}

// ============================================================================
// TaskToggle Sub-Component
// ============================================================================

/**
 * SettingsMenu.TaskToggle - checkbox for showing/hiding tasks.
 */
function TaskToggle() {
  const { state, actions } = useSettingsMenuContext()
  const { showTasks } = state
  const { toggleShowTasks } = actions

  return (
    <div className="settings-option">
      <label className="settings-checkbox">
        <input type="checkbox" checked={showTasks} onChange={toggleShowTasks} />
        <span className="settings-checkbox-text">Show tasks</span>
      </label>
    </div>
  )
}

// ============================================================================
// StyleToggle Sub-Component
// ============================================================================

/**
 * SettingsMenu.StyleToggle - radio buttons for Compact / Filled entry styles.
 */
function StyleToggle() {
  const { state, actions } = useSettingsMenuContext()
  const { entryStyle } = state
  const { setEntryStyle } = actions

  const handleChange = useCallback(
    (style: EntryStyle) => {
      setEntryStyle(style)
    },
    [setEntryStyle],
  )

  return (
    <div className="settings-option">
      <span className="settings-option-label">Style</span>
      <div className="settings-radio-group" role="radiogroup" aria-label="Entry style">
        <label className={`settings-radio${entryStyle === "compact" ? " selected" : ""}`}>
          <input
            type="radio"
            name="entryStyle"
            value="compact"
            checked={entryStyle === "compact"}
            onChange={() => handleChange("compact")}
          />
          <span className="settings-radio-text">Compact</span>
        </label>
        <label className={`settings-radio${entryStyle === "filled" ? " selected" : ""}`}>
          <input
            type="radio"
            name="entryStyle"
            value="filled"
            checked={entryStyle === "filled"}
            onChange={() => handleChange("filled")}
          />
          <span className="settings-radio-text">Filled</span>
        </label>
      </div>
    </div>
  )
}

// ============================================================================
// Main Component + Compound Export
// ============================================================================

interface SettingsMenuProps {
  /** Current view mode */
  viewMode: ViewMode
  /** Current showTasks setting */
  showTasks: boolean
  /** Current entry style */
  entryStyle: EntryStyle
  /** Callback to change view mode */
  onViewModeChange: (mode: ViewMode) => void
  /** Callback to toggle showTasks */
  onToggleShowTasks: () => void
  /** Callback to change entry style */
  onEntryStyleChange: (style: EntryStyle) => void
  /** Children (sub-components) */
  children: React.ReactNode
}

/**
 * SettingsMenu compound component.
 *
 * @example
 * <SettingsMenu
 *   viewMode={settings.viewMode}
 *   showTasks={settings.showTasks}
 *   entryStyle={settings.entryStyle}
 *   onViewModeChange={setViewMode}
 *   onToggleShowTasks={toggleShowTasks}
 *   onEntryStyleChange={setEntryStyle}
 * >
 *   <SettingsMenu.Trigger />
 *   <SettingsMenu.Panel>
 *     <SettingsMenu.ViewToggle />
 *     <SettingsMenu.StyleToggle />
 *     <SettingsMenu.TaskToggle />
 *   </SettingsMenu.Panel>
 * </SettingsMenu>
 */
function SettingsMenuRoot({
  viewMode,
  showTasks,
  entryStyle,
  onViewModeChange,
  onToggleShowTasks,
  onEntryStyleChange,
  children,
}: SettingsMenuProps) {
  return (
    <SettingsMenuProvider
      viewMode={viewMode}
      showTasks={showTasks}
      entryStyle={entryStyle}
      onViewModeChange={onViewModeChange}
      onToggleShowTasks={onToggleShowTasks}
      onEntryStyleChange={onEntryStyleChange}
    >
      <div className="settings-menu">{children}</div>
    </SettingsMenuProvider>
  )
}

// Attach sub-components as static properties
export const SettingsMenu = Object.assign(SettingsMenuRoot, {
  Trigger,
  Panel,
  ViewToggle,
  StyleToggle,
  TaskToggle,
})
