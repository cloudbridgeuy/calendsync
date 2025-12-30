/**
 * Calendar settings hook - Imperative Shell.
 * Handles localStorage persistence for view mode and task visibility settings.
 */

import {
  type CalendarSettings,
  type EntryStyle,
  getSettingsStorageKey,
  parseSettingsJson,
  serializeSettings,
  toggleShowTasks as toggleShowTasksPure,
  updateEntryStyle as updateEntryStylePure,
  updateShowTasks as updateShowTasksPure,
  updateViewMode as updateViewModePure,
  type ViewMode,
} from "@core/calendar/settings"
import { useCallback, useEffect, useState } from "react"

/** Configuration for useCalendarSettings hook */
export interface UseCalendarSettingsConfig {
  /** Calendar ID for localStorage key */
  calendarId: string
}

/** State returned by useCalendarSettings */
export interface CalendarSettingsState {
  /** Current view mode */
  viewMode: ViewMode
  /** Whether to show task entries */
  showTasks: boolean
  /** Entry color style (compact = border, filled = background) */
  entryStyle: EntryStyle
}

/** Actions returned by useCalendarSettings */
export interface CalendarSettingsActions {
  /** Set the view mode */
  setViewMode: (mode: ViewMode) => void
  /** Set the showTasks setting */
  setShowTasks: (show: boolean) => void
  /** Toggle the showTasks setting */
  toggleShowTasks: () => void
  /** Set the entry style */
  setEntryStyle: (style: EntryStyle) => void
}

/**
 * Hook to manage calendar settings with localStorage persistence.
 *
 * @param config - Hook configuration
 * @returns Tuple of [state, actions]
 */
export function useCalendarSettings(
  config: UseCalendarSettingsConfig,
): [CalendarSettingsState, CalendarSettingsActions] {
  const { calendarId } = config
  const [settings, setSettings] = useState<CalendarSettings>(() => {
    // Initialize from localStorage if available (SSR-safe)
    if (typeof window === "undefined") {
      return { viewMode: "compact", showTasks: true, entryStyle: "compact" }
    }
    const storageKey = getSettingsStorageKey(calendarId)
    const stored = localStorage.getItem(storageKey)
    return parseSettingsJson(stored)
  })

  // Persist to localStorage on change
  useEffect(() => {
    if (typeof window === "undefined") return
    const storageKey = getSettingsStorageKey(calendarId)
    const serialized = serializeSettings(settings)
    localStorage.setItem(storageKey, serialized)
  }, [calendarId, settings])

  const setViewMode = useCallback((mode: ViewMode) => {
    setSettings((prev) => updateViewModePure(prev, mode))
  }, [])

  const setShowTasks = useCallback((show: boolean) => {
    setSettings((prev) => updateShowTasksPure(prev, show))
  }, [])

  const toggleShowTasks = useCallback(() => {
    setSettings((prev) => toggleShowTasksPure(prev))
  }, [])

  const setEntryStyle = useCallback((style: EntryStyle) => {
    setSettings((prev) => updateEntryStylePure(prev, style))
  }, [])

  const state: CalendarSettingsState = {
    viewMode: settings.viewMode,
    showTasks: settings.showTasks,
    entryStyle: settings.entryStyle,
  }

  const actions: CalendarSettingsActions = {
    setViewMode,
    setShowTasks,
    toggleShowTasks,
    setEntryStyle,
  }

  return [state, actions]
}
