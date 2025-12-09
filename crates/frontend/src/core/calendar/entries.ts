/**
 * Pure entry filtering and sorting functions.
 * These functions have no side effects and are fully testable.
 */

import { formatDateKey } from "./dates"
import type { ServerDay, ServerEntry } from "./types"

/**
 * Group entries by their date key.
 * Returns a Map where keys are YYYY-MM-DD strings.
 */
export function groupEntriesByDate(entries: ServerEntry[]): Map<string, ServerEntry[]> {
  const grouped = new Map<string, ServerEntry[]>()

  for (const entry of entries) {
    const existing = grouped.get(entry.date) || []
    existing.push(entry)
    grouped.set(entry.date, existing)
  }

  return grouped
}

/**
 * Sort entries within a single day.
 * Order: all-day events first, then by start time, then by title.
 */
export function sortDayEntries(entries: ServerEntry[]): ServerEntry[] {
  return [...entries].sort((a, b) => {
    // All-day entries come first
    if (a.isAllDay && !b.isAllDay) return -1
    if (!a.isAllDay && b.isAllDay) return 1

    // Multi-day entries come after all-day but before timed
    if (a.isMultiDay && !b.isMultiDay && !b.isAllDay) return -1
    if (!a.isMultiDay && b.isMultiDay && !a.isAllDay) return 1

    // Sort by start time if both have times
    if (a.startTime && b.startTime) {
      const comparison = a.startTime.localeCompare(b.startTime)
      if (comparison !== 0) return comparison
    }

    // Finally sort by title
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
    if (entry.date === dateKey) return true

    // Multi-day case: check if dateKey falls within the span
    if (entry.isMultiDay && entry.multiDayStartDate && entry.multiDayEndDate) {
      return dateKey >= entry.multiDayStartDate && dateKey <= entry.multiDayEndDate
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
