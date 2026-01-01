/**
 * Pure transformation functions for offline-first calendar sync.
 * These functions have no side effects and are fully testable.
 */

import type { EntryFormData } from "../calendar/modal"
import type { ServerEntry } from "../calendar/types"
import type { LocalEntry } from "./types"

/**
 * Entry type flags derived from EntryFormData.
 */
export interface DerivedEntryType {
  isAllDay: boolean
  isTimed: boolean
  isTask: boolean
  isMultiDay: boolean
}

/**
 * Derive entry type flags from form data.
 *
 * @param data - The entry form data
 * @returns Object with boolean flags for each entry type
 */
export function deriveEntryType(data: EntryFormData): DerivedEntryType {
  return {
    isAllDay: data.entryType === "all_day",
    isTimed: data.entryType === "timed",
    isTask: data.entryType === "task",
    isMultiDay: data.entryType === "multi_day",
  }
}

/**
 * Convert EntryFormData to a partial ServerEntry for local storage.
 *
 * @param data - The entry form data
 * @param calendarId - The calendar ID
 * @param existingEntry - Optional existing entry to preserve color from
 * @returns A partial ServerEntry (without id) ready for local storage
 */
export function formDataToEntry(
  data: EntryFormData,
  calendarId: string,
  existingEntry?: LocalEntry,
): Omit<ServerEntry, "id"> {
  const types = deriveEntryType(data)

  return {
    calendarId,
    kind: data.entryType,
    completed: data.completed ?? false,
    isMultiDay: types.isMultiDay,
    isAllDay: types.isAllDay,
    isTimed: types.isTimed,
    isTask: types.isTask,
    title: data.title,
    description: data.description ?? null,
    location: data.location ?? null,
    color: existingEntry?.color ?? null,
    startDate: data.startDate,
    endDate: data.endDate ?? data.startDate,
    startTime: data.startTime ?? null,
    endTime: data.endTime ?? null,
  }
}
