/**
 * Pure functions for calendar settings management.
 */

// ============================================================================
// Types
// ============================================================================

/** View mode for the calendar display */
export type ViewMode = "compact" | "schedule"

/** Entry color style for rendering */
export type EntryStyle = "compact" | "filled"

/** Calendar settings stored per calendar */
export interface CalendarSettings {
  /** Current view mode */
  viewMode: ViewMode
  /** Whether to show task entries */
  showTasks: boolean
  /** Entry color style (compact = border, filled = background) */
  entryStyle: EntryStyle
}

// ============================================================================
// Constants
// ============================================================================

/** Default settings for new calendars */
export const DEFAULT_SETTINGS: CalendarSettings = {
  viewMode: "compact",
  showTasks: true,
  entryStyle: "compact",
}

// ============================================================================
// Update Functions (Pure)
// ============================================================================

/**
 * Create new settings with updated view mode.
 */
export function updateViewMode(settings: CalendarSettings, viewMode: ViewMode): CalendarSettings {
  return { ...settings, viewMode }
}

/**
 * Create new settings with updated showTasks.
 */
export function updateShowTasks(settings: CalendarSettings, showTasks: boolean): CalendarSettings {
  return { ...settings, showTasks }
}

/**
 * Create new settings with toggled showTasks.
 */
export function toggleShowTasks(settings: CalendarSettings): CalendarSettings {
  return { ...settings, showTasks: !settings.showTasks }
}

/**
 * Create new settings with updated entry style.
 */
export function updateEntryStyle(
  settings: CalendarSettings,
  entryStyle: EntryStyle,
): CalendarSettings {
  return { ...settings, entryStyle }
}
