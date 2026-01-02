/**
 * Pure entry filtering and sorting functions.
 * These functions have no side effects and are fully testable.
 */

import { dateRangeStrings, formatDateKey, maxDateString, minDateString } from "./dates"
import type { ServerDay, ServerEntry } from "./types"

/**
 * Group entries by their date key.
 * Returns a Map where keys are YYYY-MM-DD strings.
 *
 * This function is generic and preserves the entry type, so if you pass
 * LocalEntry[], you get Map<string, LocalEntry[]> back.
 */
export function groupEntriesByDate<T extends ServerEntry>(entries: T[]): Map<string, T[]> {
  const grouped = new Map<string, T[]>()

  for (const entry of entries) {
    const existing = grouped.get(entry.startDate) || []
    existing.push(entry)
    grouped.set(entry.startDate, existing)
  }

  return grouped
}

/**
 * Sort entries within a single day for compact view.
 * Order: multi-day > all-day > tasks > timed (by start time) > alphabetical by title
 */
export function sortDayEntries(entries: ServerEntry[]): ServerEntry[] {
  return [...entries].sort((a, b) => {
    // 1. Multi-day entries come first
    if (a.isMultiDay && !b.isMultiDay) return -1
    if (!a.isMultiDay && b.isMultiDay) return 1

    // 2. All-day entries come second
    if (a.isAllDay && !b.isAllDay) return -1
    if (!a.isAllDay && b.isAllDay) return 1

    // 3. Tasks come third (before timed)
    if (a.isTask && !b.isTask) return -1
    if (!a.isTask && b.isTask) return 1

    // 4. Timed entries: sort by start time
    if (a.startTime && b.startTime) {
      const comparison = a.startTime.localeCompare(b.startTime)
      if (comparison !== 0) return comparison
    }

    // 5. Tie-breaker: alphabetical by title
    return a.title.localeCompare(b.title)
  })
}

/**
 * Filter entries to only those that should appear on a given date.
 * This handles multi-day entries that span across dates.
 */
export function getEntriesForDate(entries: ServerEntry[], dateKey: string): ServerEntry[] {
  return entries.filter((entry) => {
    // Simple case: entry's date matches
    if (entry.startDate === dateKey) return true

    // Multi-day case: check if dateKey falls within the span
    if (entry.isMultiDay) {
      return dateKey >= entry.startDate && dateKey <= entry.endDate
    }

    return false
  })
}

/**
 * Convert ServerDay array to a Map for quick lookup.
 */
export function serverDaysToMap(days: ServerDay[]): Map<string, ServerEntry[]> {
  const map = new Map<string, ServerEntry[]>()
  for (const day of days) {
    map.set(day.date, day.entries)
  }
  return map
}

/**
 * Merge new entries into existing cache.
 * New entries for a date replace existing entries for that date.
 */
export function mergeEntryCache(
  existing: Map<string, ServerEntry[]>,
  newDays: ServerDay[],
): Map<string, ServerEntry[]> {
  const merged = new Map(existing)
  for (const day of newDays) {
    merged.set(day.date, day.entries)
  }
  return merged
}

/**
 * Get date keys that are missing from the cache.
 */
export function getMissingDateKeys(
  cache: Map<string, ServerEntry[]>,
  dateKeys: string[],
): string[] {
  return dateKeys.filter((key) => !cache.has(key))
}

/**
 * Find the date range needed for a given center date and visible day count.
 * Returns { start: string, end: string } in YYYY-MM-DD format.
 */
export function getRequiredDateRange(
  centerDate: Date,
  visibleDays: number,
  bufferDays: number = 7,
): { start: string; end: string } {
  const halfVisible = Math.floor(visibleDays / 2)
  const startOffset = halfVisible + bufferDays
  const endOffset = halfVisible + bufferDays

  const startDate = new Date(centerDate)
  startDate.setDate(startDate.getDate() - startOffset)

  const endDate = new Date(centerDate)
  endDate.setDate(endDate.getDate() + endOffset)

  return {
    start: formatDateKey(startDate),
    end: formatDateKey(endDate),
  }
}

/**
 * Check if an entry is a task (vs an event).
 */
export function isTaskEntry(entry: ServerEntry): boolean {
  return entry.isTask
}

/**
 * Check if an entry is completed (for tasks).
 */
export function isCompletedEntry(entry: ServerEntry): boolean {
  return entry.completed
}

/**
 * Filter entries by completion status.
 */
export function filterByCompletion(entries: ServerEntry[], completed: boolean): ServerEntry[] {
  return entries.filter((entry) => entry.completed === completed)
}

/**
 * Filter entries by calendar ID.
 */
export function filterByCalendar(entries: ServerEntry[], calendarId: string): ServerEntry[] {
  return entries.filter((entry) => entry.calendarId === calendarId)
}

/**
 * Get unique calendar IDs from a list of entries.
 */
export function getUniqueCalendarIds(entries: ServerEntry[]): string[] {
  const ids = new Set<string>()
  for (const entry of entries) {
    ids.add(entry.calendarId)
  }
  return Array.from(ids)
}

/**
 * Filter entries by task visibility setting.
 * When showTasks is false, removes all task entries.
 */
export function filterByTaskVisibility(entries: ServerEntry[], showTasks: boolean): ServerEntry[] {
  if (showTasks) {
    return entries
  }
  return entries.filter((entry) => !entry.isTask)
}

/**
 * Filter entries to only timed events (not all-day, multi-day, or tasks).
 * Used for schedule mode timed grid.
 */
export function filterTimedEntries(entries: ServerEntry[]): ServerEntry[] {
  return entries.filter((e) => !e.isAllDay && !e.isMultiDay && !e.isTask)
}

/**
 * Filter entries to only all-day, multi-day, and tasks.
 * Used for schedule mode all-day section.
 */
export function filterAllDayEntries(entries: ServerEntry[]): ServerEntry[] {
  return entries.filter((e) => e.isAllDay || e.isMultiDay || e.isTask)
}

/**
 * Expand multi-day entries into a map of date -> entries.
 * Multi-day entries appear on every day they span (clipped to view bounds).
 * Single-day entries appear only on their start date.
 */
export function expandMultiDayEntries(
  entries: ServerEntry[],
  viewStart: string,
  viewEnd: string,
): Map<string, ServerEntry[]> {
  const dayMap = new Map<string, ServerEntry[]>()

  for (const entry of entries) {
    if (entry.isMultiDay) {
      // Clip to view bounds
      const start = maxDateString(entry.startDate, viewStart)
      const end = minDateString(entry.endDate, viewEnd)

      for (const date of dateRangeStrings(start, end)) {
        addToDay(dayMap, date, entry)
      }
    } else {
      // Single-day entry
      if (entry.startDate >= viewStart && entry.startDate <= viewEnd) {
        addToDay(dayMap, entry.startDate, entry)
      }
    }
  }

  return dayMap
}

function addToDay(map: Map<string, ServerEntry[]>, date: string, entry: ServerEntry): void {
  const existing = map.get(date) ?? []
  existing.push(entry)
  map.set(date, existing)
}
