/**
 * Core calendar module - pure functions with no side effects.
 * This is the Functional Core of the calendar application.
 */

// Re-export ARIA functions
export type { AriaIds } from "./aria"
export { buildAriaIds } from "./aria"
// Re-export date functions
export {
  addDays,
  formatDateKey,
  getDateRange,
  getDatesAround,
  getDayOfMonth,
  getDayOfWeek,
  getMonth,
  getYear,
  isSameDay,
  isToday,
  parseDateKey,
  startOfDay,
} from "./dates"
// Re-export entry functions
export {
  filterByCalendar,
  filterByCompletion,
  getEntriesForDate,
  getMissingDateKeys,
  getRequiredDateRange,
  getUniqueCalendarIds,
  groupEntriesByDate,
  isCompletedEntry,
  isTaskEntry,
  mergeEntryCache,
  serverDaysToMap,
  sortDayEntries,
} from "./entries"
// Re-export layout functions
export {
  calculateAnimationDuration,
  calculateDayPosition,
  calculateDaysFromWheelDelta,
  calculateDayWidth,
  calculateOffsetFromCenter,
  calculateSwipeTransform,
  calculateVisibleDays,
  calculateWheelDragOffset,
  detectWheelDirection,
  getVisibleDateOffsets,
  getWheelNavigationDelta,
  isMobileViewport,
  shouldLoadMoreDays,
  shouldNavigateFromSwipe,
  snapToNearestDay,
} from "./layout"
export type { EntryFormData, ParsedModalUrl, ValidationResult } from "./modal"
// Re-export modal functions
export {
  buildCalendarUrl,
  buildModalUrl,
  createDefaultFormData,
  entryToFormData,
  FOCUSABLE_SELECTOR,
  formDataToApiPayload,
  getNextFocusIndex,
  parseModalUrl,
  validateFormData,
} from "./modal"
// Re-export notification functions
export {
  addNotification,
  countUnread,
  createNotification,
  formatNotificationTime,
  getNotificationIcon,
  getNotificationLabel,
  getStorageKey,
  MAX_NOTIFICATIONS,
  markAllNotificationsAsRead,
  markNotificationAsRead,
  NOTIFICATION_STORAGE_PREFIX,
  parseNotificationsJson,
  removeNotification,
  serializeNotifications,
} from "./notifications"
// Re-export all types
export type {
  LayoutConstants,
  ServerDay,
  ServerEntry,
} from "./types"
export {
  DAY_NAMES,
  DAY_NAMES_FULL,
  DEFAULT_LAYOUT_CONSTANTS,
  MONTH_NAMES,
} from "./types"
