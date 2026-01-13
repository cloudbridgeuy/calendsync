/**
 * Core calendar module - pure functions with no side effects.
 * This is the Functional Core of the calendar application.
 */

// Re-export accessibility functions
export {
  buildDayColumnSelector,
  buildDayHeaderSelector,
  buildFirstEntrySelector,
  focusDayElement,
  generateNavigationAnnouncement,
} from "./accessibility"
// Re-export all-day layout functions
export type { AllDayCategorized, AllDaySummary } from "./allDayLayout"
export {
  categorizeAllDayEntries,
  computeAllDaySummary,
  formatOverflowToggle,
  formatTasksToggle,
  MAX_VISIBLE_ALL_DAY,
} from "./allDayLayout"
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
  getTimezoneAbbreviation,
  getYear,
  isSameDay,
  isToday,
  parseDateKey,
  startOfDay,
} from "./dates"
// Re-export day container functions
export type { DayDisplayInfo } from "./dayContainer"
export { getDayDisplayInfo, isDayToday } from "./dayContainer"
// Re-export entry functions
export {
  deriveEntryTypeFromFlags,
  expandMultiDayEntries,
  filterAllDayEntries,
  filterByCalendar,
  filterByCompletion,
  filterByTaskVisibility,
  filterTimedEntries,
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
// Re-export form data conversion functions
export { createPayloadToFormData, updatePayloadToFormData } from "./forms"
// Re-export layout functions
export {
  calculateAnimationDuration,
  calculateDayPosition,
  calculateDaysFromWheelDelta,
  calculateOffsetFromCenter,
  calculateSwipeTransform,
  calculateWheelDragOffset,
  detectWheelDirection,
  getVisibleDateOffsets,
  getWheelNavigationDelta,
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
// Re-export navigation functions
export {
  calculateCenterDayIndex,
  calculateCenteredScrollPosition,
  detectEdgeProximity,
  isScrollable,
} from "./navigation"
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
// Re-export schedule layout functions
export type {
  OverlapColumn,
  SeparatedEntries,
  TimePosition,
  TimePositionPercent,
} from "./scheduleLayout"
export {
  calculateDuration,
  calculateEntryWidth,
  calculateGridHeight,
  calculateHourLinePositionPercent,
  calculateScrollToHour,
  calculateTimePosition,
  calculateTimePositionPercent,
  DEFAULT_SCROLL_HOUR,
  detectOverlappingEntries,
  formatHourLabel,
  generateHourLabels,
  getOverlappingEntries,
  HOUR_HEIGHT_PX,
  HOURS_IN_DAY,
  MINUTES_IN_DAY,
  parseTimeToMinutes,
  separateEntriesByType,
} from "./scheduleLayout"
// Re-export scroll animation functions
export type {
  EasingFunction,
  ScrollAnimationConfig,
  ScrollAnimationState,
} from "./scrollAnimation"
export {
  calculateAnimationProgress,
  calculateCurrentScrollPosition,
  calculateScaledDuration,
  calculateScrollDistance,
  createAnimationState,
  DEFAULT_SCROLL_ANIMATION_CONFIG,
  easeOutCubic,
  isAnimationComplete,
} from "./scrollAnimation"
// Re-export selection functions
export type { CalendarSelection, InitialView } from "./selection"
export { determineInitialView, selectCalendar } from "./selection"
// Re-export settings functions
export type { CalendarSettings, EntryStyle, ViewMode } from "./settings"
export {
  DEFAULT_SETTINGS,
  getSettingsStorageKey,
  parseSettingsJson,
  SETTINGS_STORAGE_PREFIX,
  serializeSettings,
  toggleShowTasks,
  updateEntryStyle,
  updateShowTasks,
  updateViewMode,
} from "./settings"
// Re-export all types
export type { LayoutConstants, ServerDay, ServerEntry } from "./types"
export { DAY_NAMES, DAY_NAMES_FULL, DEFAULT_LAYOUT_CONSTANTS, MONTH_NAMES } from "./types"
// Re-export virtual scroll functions
export type { RecenterResult, VirtualScrollConfig } from "./virtualScroll"
export {
  calculateDayIndex,
  calculateDayWidth,
  calculateHighlightedDay,
  calculateRecenterOffset,
  calculateScrollPosition,
  calculateTotalWidth,
  calculateVirtualWindow,
  calculateVisibleDays,
  calculateWindowDates,
  calculateWindowStartDate,
  DEFAULT_VIRTUAL_SCROLL_CONFIG,
  isSameCalendarDay,
  shouldRecenter,
} from "./virtualScroll"
