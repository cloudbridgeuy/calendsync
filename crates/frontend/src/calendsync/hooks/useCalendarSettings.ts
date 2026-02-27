/**
 * Calendar settings hook - Imperative Shell.
 * Reads initial settings from server data and saves changes via debounced PUT.
 */

import {
  type CalendarSettings,
  DEFAULT_SETTINGS,
  type EntryStyle,
  toggleShowTasks as toggleShowTasksPure,
  updateEntryStyle as updateEntryStylePure,
  updateShowTasks as updateShowTasksPure,
  updateViewMode as updateViewModePure,
  type ViewMode,
} from "@core/calendar/settings"
import { useCallback, useEffect, useRef, useState } from "react"

/** Configuration for useCalendarSettings hook */
export interface UseCalendarSettingsConfig {
  /** Initial settings from server (may be undefined if no-auth) */
  initialSettings?: CalendarSettings
  /** Calendar ID for the save endpoint */
  calendarId: string
  /** Base URL for API calls */
  controlPlaneUrl: string
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
 * Hook to manage calendar settings with debounced server persistence.
 *
 * Reads initial settings from server data (via initialData.settings).
 * On change, optimistically updates local state and debounces a PUT
 * request to save settings on the server (fire-and-forget).
 *
 * @param config - Hook configuration
 * @returns Tuple of [state, actions]
 */
export function useCalendarSettings(
  config: UseCalendarSettingsConfig,
): [CalendarSettingsState, CalendarSettingsActions] {
  const { initialSettings, calendarId, controlPlaneUrl } = config

  const [settings, setSettings] = useState<CalendarSettings>(
    () => initialSettings ?? DEFAULT_SETTINGS,
  )

  // Track whether this is the first render (don't save initial settings back)
  const isFirstRender = useRef(true)

  // Debounced save to server when settings change
  useEffect(() => {
    if (isFirstRender.current) {
      isFirstRender.current = false
      return
    }

    const timer = setTimeout(() => {
      fetch(`${controlPlaneUrl}/api/calendars/${calendarId}/settings`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify(settings),
      }).catch(() => {
        // Fire-and-forget: swallow errors silently
      })
    }, 500)

    return () => clearTimeout(timer)
  }, [settings, calendarId, controlPlaneUrl])

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
