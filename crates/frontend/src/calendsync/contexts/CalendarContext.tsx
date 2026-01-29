/**
 * CalendarContext - provides shared state to calendar sub-components.
 * This eliminates props drilling through DayColumn to EntryTile.
 */

import type { EntryStyle, ViewMode } from "@core/calendar/settings"
import type { ServerEntry } from "@core/calendar/types"
import type { SseConnectionState } from "@core/sse/types"
import type { LocalEntry } from "@core/sync/types"
import { createContext, useContext } from "react"
import type {
  AddNotificationFn,
  CalendarSettingsState,
  NotificationCenterActions,
  NotificationCenterState,
} from "../hooks"
import type { ChangeType, ModalState, UserInfo } from "../types"

/** Context value shared with calendar sub-components */
export interface CalendarContextValue {
  // Existing - entry display
  /** Map of entry IDs to their flash animation state */
  flashStates: Map<string, ChangeType>
  /** Callback when an entry is clicked (opens edit modal) */
  onEntryClick: (entry: ServerEntry) => void
  /** Callback when a task entry checkbox is toggled */
  onEntryToggle: (entry: ServerEntry) => void

  // Calendar state
  /** The current center/highlighted date */
  centerDate: Date
  /** Number of visible days (1 for mobile, 3/5/7 for desktop) */
  visibleDays: number
  /** Cache of entries by date key (YYYY-MM-DD) */
  entryCache: Map<string, ServerEntry[]>
  /** SSE connection state for real-time updates */
  sseConnectionState: SseConnectionState
  /** Error message if data loading failed */
  error: string | null

  // Virtual scroll state
  /** Ref to attach to the scroll container */
  scrollContainerRef: React.RefObject<HTMLDivElement | null>
  /** Currently highlighted date (center of viewport) */
  highlightedDate: Date
  /** Array of dates currently rendered in virtual window */
  renderedDates: Date[]
  /** Width of each day column in pixels */
  dayWidth: number

  // Navigation actions
  /** Scroll to a specific date */
  scrollToDate: (date: Date, animated?: boolean) => void
  /** Navigate by a number of days (positive = forward, negative = back) */
  navigateDays: (offset: number) => void
  /** Jump to today */
  goToToday: () => void
  /** Get entries for a specific date */
  getEntriesForDate: (date: Date) => ServerEntry[]
  /** Reconnect to SSE stream */
  refresh: () => void

  // New - modal state
  /** Open create modal for a specific date */
  openCreateModal: (date: string) => void
  /** Open edit modal for a specific entry */
  openEditModal: (entryId: string) => void
  /** Close the modal */
  closeModal: () => void
  /** Current modal state (null if closed) */
  modalState: ModalState | null
  /** Edit entry being displayed in modal (from SSR, cache, or fetched) */
  editEntry: ServerEntry | undefined
  /** Handle modal save */
  handleModalSave: (savedEntry: ServerEntry) => void
  /** Handle modal delete */
  handleModalDelete: () => void
  /** Calendar ID for API calls */
  calendarId: string

  // New - notification state
  /** Notification center state */
  notificationState: NotificationCenterState
  /** Notification center actions */
  notificationActions: NotificationCenterActions
  /** Add a new notification */
  addNotification: AddNotificationFn

  // Settings state
  /** Calendar settings (view mode, task visibility, entry style) */
  settings: CalendarSettingsState
  /** Set the view mode */
  setViewMode: (mode: ViewMode) => void
  /** Set the showTasks setting */
  setShowTasks: (show: boolean) => void
  /** Toggle the showTasks setting */
  toggleShowTasks: () => void
  /** Set the entry color style */
  setEntryStyle: (style: EntryStyle) => void

  // All-day section toggle states
  /** Whether to show overflow entries in all-day section */
  showAllDayOverflow: boolean
  /** Toggle overflow entries visibility */
  setShowAllDayOverflow: (show: boolean) => void
  /** Whether to show task checkboxes in all-day section */
  showAllDayTasks: boolean
  /** Toggle task checkboxes visibility */
  setShowAllDayTasks: (show: boolean) => void

  // Offline sync state
  /** Whether the browser is currently online */
  isOnline: boolean
  /** Number of pending operations waiting to sync */
  pendingCount: number
  /** Whether sync is currently in progress */
  isSyncing: boolean
  /** Whether offline mode is enabled (entries from Dexie) */
  offlineEnabled: boolean
  /** Get entry with local sync status (for displaying pending/conflict indicators) */
  getLocalEntry: (entryId: string) => LocalEntry | undefined

  // User info
  /** Logged-in user info (from SSR) */
  user?: UserInfo
}

/** CalendarContext - null when not inside provider */
const CalendarContext = createContext<CalendarContextValue | null>(null)

/** Props for CalendarProvider */
export interface CalendarProviderProps {
  children: React.ReactNode
  value: CalendarContextValue
}

/**
 * CalendarProvider - wraps calendar sub-components with shared context.
 */
export function CalendarProvider({ children, value }: CalendarProviderProps) {
  return <CalendarContext.Provider value={value}>{children}</CalendarContext.Provider>
}

/**
 * Hook to access CalendarContext.
 * Throws if used outside CalendarProvider.
 */
export function useCalendarContext(): CalendarContextValue {
  const ctx = useContext(CalendarContext)
  if (!ctx) {
    throw new Error("useCalendarContext must be used within CalendarProvider")
  }
  return ctx
}
