/**
 * React Calendar Types
 * Extends and re-exports core calendar types with React-specific additions.
 */

// Re-export core types
export type {
  LayoutConstants,
  ServerDay,
  ServerEntry,
} from "@core/calendar/types"

export {
  DAY_NAMES,
  DAY_NAMES_FULL,
  DEFAULT_LAYOUT_CONSTANTS,
  MONTH_NAMES,
} from "@core/calendar/types"

import type { SseConnectionState } from "@core/sse/types"

/**
 * User info from SSR authentication.
 */
export interface UserInfo {
  /** User's display name */
  name: string
  /** User's email address */
  email: string
}

/**
 * Modal state from SSR or URL parsing.
 * Determines whether the entry modal is open and in what mode.
 */
export interface ModalState {
  /** Modal mode: create new entry or edit existing */
  mode: "create" | "edit"
  /** Entry ID for edit mode */
  entryId?: string
  /** Pre-fetched entry data for edit mode (from SSR) */
  entry?: import("@core/calendar/types").ServerEntry
  /** Default date for create mode (pre-fill the date field) */
  defaultDate?: string
}

// Re-export EntryFormData from core modal utilities
export type { EntryFormData } from "@core/calendar/modal"

/**
 * Initial data passed from server to client via __INITIAL_DATA__
 */
export interface InitialData {
  /** The calendar ID from the URL */
  calendarId: string
  /** The highlighted/center date (ISO 8601 format: YYYY-MM-DD) */
  highlightedDay: string
  /** Initial entries grouped by date */
  days: import("@core/calendar/types").ServerDay[]
  /** URL to the client bundle (hashed) */
  clientBundleUrl: string
  /** URL to the CSS bundle (hashed) */
  cssBundleUrl?: string
  /** Base URL for API calls */
  controlPlaneUrl: string
  /** Modal state from SSR (if modal URL was requested) */
  modal?: ModalState
  /** Whether dev mode is enabled (for hot-reload auto-refresh) */
  devMode?: boolean
  /** Whether SSE real-time updates are enabled (default: true, false for Tauri) */
  sseEnabled?: boolean
  /** Logged-in user info (from SSR) */
  user?: UserInfo
}

/**
 * Calendar component props
 */
export interface CalendarProps {
  /** Initial data from SSR */
  initialData: InitialData
}

/** Entry change type for flash animations */
export type EntryChangeType = "added" | "updated" | "deleted"

/** Toast notification for entry changes */
export interface Toast {
  id: string
  type: EntryChangeType
  title: string
  timestamp: number
}

/** Flash state for entry animations */
export type FlashState = "added" | "updated" | "deleted"

/** Notification type for notification center */
export type NotificationType = "added" | "updated" | "deleted"

/** Notification stored in the notification center */
export interface Notification {
  /** Unique notification ID */
  id: string
  /** Type of change */
  type: NotificationType
  /** ID of the entry that changed */
  entryId: string
  /** Title of the entry that changed */
  entryTitle: string
  /** Date the entry is on (YYYY-MM-DD) */
  date: string
  /** Timestamp when the notification was created */
  timestamp: number
  /** Whether the notification has been read/acknowledged */
  read: boolean
}

/** Toast notification data */
export interface ToastData {
  id: string
  type: FlashState
  title: string
  date: string
}

/**
 * Calendar state managed by useCalendarState hook
 */
export interface CalendarState {
  /** The current center/highlighted date */
  centerDate: Date
  /** Number of visible days (1 for mobile, 3/5/7 for desktop) */
  visibleDays: number
  /** Cache of entries by date key (YYYY-MM-DD) */
  entryCache: Map<string, import("@core/calendar/types").ServerEntry[]>
  /** SSE connection state for real-time updates */
  sseConnectionState: SseConnectionState
  /** Error message if data loading failed */
  error: string | null
  /** Map of entry IDs to their flash animation state */
  flashStates: Map<string, FlashState>
  /** Active toast notifications */
  toasts: ToastData[]
}

/**
 * Calendar actions for state updates
 */
export interface CalendarActions {
  /** Navigate by a number of days (positive = forward, negative = back) */
  navigateDays: (offset: number) => void
  /** Jump to today */
  goToToday: () => void
  /** Jump to a specific date */
  goToDate: (date: Date) => void
  /** Update layout based on viewport width */
  updateLayout: (width: number) => void
  /** Remove a toast notification */
  removeToast: (id: string) => void
  /** Add entry to cache optimistically (before SSE confirmation) */
  addEntryOptimistic: (entry: import("@core/calendar/types").ServerEntry) => void
  /** Update entry in cache optimistically (before SSE confirmation) */
  updateEntryOptimistic: (entry: import("@core/calendar/types").ServerEntry) => void
  /** Handle SSE entry_added event with visual feedback (flash, toast, notification) */
  onSseEntryAdded: (entry: import("@core/calendar/types").ServerEntry, date: string) => void
  /** Handle SSE entry_updated event with visual feedback (flash, toast, notification) */
  onSseEntryUpdated: (entry: import("@core/calendar/types").ServerEntry, date: string) => void
  /** Handle SSE entry_deleted event with visual feedback (flash, toast, notification) */
  onSseEntryDeleted: (entryId: string, date: string) => void
  /** Handle SSE connection state change */
  onSseConnectionChange: (state: SseConnectionState) => void
}

/**
 * SSR configuration passed via globalThis.__SSR_CONFIG__
 */
export interface SSRConfig {
  /** Initial data for the calendar */
  initialData: InitialData
}

/**
 * Declare global types for SSR and client
 */
declare global {
  interface Window {
    __INITIAL_DATA__: InitialData
  }

  // eslint-disable-next-line no-var
  var __SSR_CONFIG__: SSRConfig | undefined
}
