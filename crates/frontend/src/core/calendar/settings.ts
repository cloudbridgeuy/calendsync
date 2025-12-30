/**
 * Pure functions for calendar settings management.
 * Settings are persisted to localStorage per calendar.
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

/** localStorage key prefix for calendar settings */
export const SETTINGS_STORAGE_PREFIX = "calendsync_settings_"

/** Default settings for new calendars */
export const DEFAULT_SETTINGS: CalendarSettings = {
  viewMode: "compact",
  showTasks: true,
  entryStyle: "compact",
}

// ============================================================================
// Storage Key Functions
// ============================================================================

/**
 * Get the localStorage key for a calendar's settings.
 */
export function getSettingsStorageKey(calendarId: string): string {
  return `${SETTINGS_STORAGE_PREFIX}${calendarId}`
}

// ============================================================================
// Serialization Functions
// ============================================================================

/**
 * Parse settings JSON from localStorage.
 * Returns default settings if JSON is null or invalid.
 */
export function parseSettingsJson(json: string | null): CalendarSettings {
  if (!json) {
    return { ...DEFAULT_SETTINGS }
  }

  try {
    const parsed = JSON.parse(json)

    // Validate and extract settings with defaults
    const viewMode: ViewMode =
      parsed.viewMode === "compact" || parsed.viewMode === "schedule"
        ? parsed.viewMode
        : DEFAULT_SETTINGS.viewMode

    const showTasks: boolean =
      typeof parsed.showTasks === "boolean" ? parsed.showTasks : DEFAULT_SETTINGS.showTasks

    const entryStyle: EntryStyle =
      parsed.entryStyle === "compact" || parsed.entryStyle === "filled"
        ? parsed.entryStyle
        : DEFAULT_SETTINGS.entryStyle

    return { viewMode, showTasks, entryStyle }
  } catch {
    return { ...DEFAULT_SETTINGS }
  }
}

/**
 * Serialize settings to JSON string for localStorage.
 */
export function serializeSettings(settings: CalendarSettings): string {
  return JSON.stringify(settings)
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
