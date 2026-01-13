/**
 * Pure functions for converting calendar entry data to API-compatible form data.
 * This module handles the transformation between different entry representations and URLSearchParams.
 */

import type { CreateEntryPayload } from "../transport/types"
import { deriveEntryTypeFromFlags } from "./entries"
import { entryToFormData, formDataToApiPayload } from "./modal"
import type { ServerEntry } from "./types"

/**
 * Convert CreateEntryPayload to URLSearchParams for API submission.
 * This is the primary function for creating new entries via the API.
 *
 * @param payload - The entry creation payload
 * @returns URLSearchParams ready for API submission
 */
export function createPayloadToFormData(payload: CreateEntryPayload): URLSearchParams {
  const params = new URLSearchParams()

  params.set("calendar_id", payload.calendar_id)
  params.set("title", payload.title)
  params.set("start_date", payload.date)

  // Determine entry_type
  if (payload.entry_type) {
    params.set("entry_type", payload.entry_type)
  } else if (payload.all_day) {
    params.set("entry_type", "all_day")
  } else if (payload.start_time || payload.end_time) {
    params.set("entry_type", "timed")
  } else {
    params.set("entry_type", "all_day")
  }

  if (payload.start_time) {
    params.set("start_time", payload.start_time)
  }
  if (payload.end_time) {
    params.set("end_time", payload.end_time)
  }
  if (payload.description) {
    params.set("description", payload.description)
  }

  return params
}

/**
 * Convert a partial ServerEntry to URLSearchParams for API submission.
 * This is used by the sync engine for update operations.
 *
 * If the payload is a complete ServerEntry, it uses the existing conversion.
 * For partial payloads, it builds form data manually.
 *
 * @param payload - Partial or complete ServerEntry
 * @param calendarId - The calendar ID to include in the form data
 * @returns URLSearchParams ready for API submission
 */
export function updatePayloadToFormData(
  payload: Partial<ServerEntry>,
  calendarId: string,
): URLSearchParams {
  // If we have a complete ServerEntry, use the existing conversion
  if (isCompleteEntry(payload)) {
    const formData = entryToFormData(payload as ServerEntry)
    return formDataToApiPayload(formData, calendarId)
  }

  // For partial payloads, build form data manually
  const entryType = deriveEntryTypeFromFlags(payload)
  const formData = {
    title: payload.title ?? "",
    startDate: payload.startDate ?? "",
    endDate: payload.endDate,
    isAllDay: payload.isAllDay ?? entryType === "all_day",
    description: payload.description ?? undefined,
    location: payload.location ?? undefined,
    entryType,
    startTime: payload.startTime ?? undefined,
    endTime: payload.endTime ?? undefined,
    completed: payload.completed,
  }

  return formDataToApiPayload(formData, calendarId)
}

/**
 * Check if a partial ServerEntry has enough fields to be considered complete.
 * A complete entry has all required fields: id, title, startDate, and isAllDay.
 *
 * @param payload - Partial ServerEntry to check
 * @returns true if the payload is a complete entry
 */
function isCompleteEntry(payload: Partial<ServerEntry>): boolean {
  return (
    typeof payload.id === "string" &&
    typeof payload.title === "string" &&
    typeof payload.startDate === "string" &&
    typeof payload.isAllDay === "boolean"
  )
}
