/**
 * Core calendar module - pure functions with no side effects.
 * This is the Functional Core of the calendar application.
 */

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
    calculateDayWidth,
    calculateOffsetFromCenter,
    calculateSwipeTransform,
    calculateVisibleDays,
    getVisibleDateOffsets,
    isMobileViewport,
    shouldLoadMoreDays,
    shouldNavigateFromSwipe,
    snapToNearestDay,
} from "./layout"
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
