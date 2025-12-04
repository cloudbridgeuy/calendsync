/**
 * Pure modal utilities for URL parsing and form data manipulation.
 * Functional Core: No side effects, no DOM access, no I/O operations.
 */

import type { ServerEntry } from "./types"

/**
 * Parsed modal URL result.
 */
export interface ParsedModalUrl {
    mode: "create" | "edit"
    entryId?: string
}

/**
 * Entry form data for creating/editing entries.
 */
export interface EntryFormData {
    title: string
    date: string
    startTime?: string
    endTime?: string
    isAllDay: boolean
    description?: string
    location?: string
    entryType: "all_day" | "timed" | "task" | "multi_day"
    endDate?: string
}

/**
 * Form validation result.
 */
export interface ValidationResult {
    valid: boolean
    errors: string[]
}

/**
 * Parse a modal URL to extract mode and entry ID.
 * Returns null if not a modal URL.
 *
 * @example
 * parseModalUrl("/calendar/abc-123/entry", "") // { mode: "create" }
 * parseModalUrl("/calendar/abc-123/entry", "?entry_id=xyz-789") // { mode: "edit", entryId: "xyz-789" }
 * parseModalUrl("/calendar/abc-123", "") // null
 */
export function parseModalUrl(pathname: string, search: string): ParsedModalUrl | null {
    // Match /calendar/{calendar_id}/entry
    const match = pathname.match(/^\/calendar\/[^/]+\/entry$/)
    if (!match) {
        return null
    }

    // Parse query params for entry_id
    const params = new URLSearchParams(search)
    const entryId = params.get("entry_id")

    if (entryId) {
        return { mode: "edit", entryId }
    }

    return { mode: "create" }
}

/**
 * Build a modal URL for the given calendar and mode.
 *
 * @example
 * buildModalUrl("abc-123", "create") // "/calendar/abc-123/entry"
 * buildModalUrl("abc-123", "edit", "xyz-789") // "/calendar/abc-123/entry?entry_id=xyz-789"
 */
export function buildModalUrl(
    calendarId: string,
    mode: "create" | "edit",
    entryId?: string,
): string {
    const basePath = `/calendar/${calendarId}/entry`

    if (mode === "edit" && entryId) {
        return `${basePath}?entry_id=${encodeURIComponent(entryId)}`
    }

    return basePath
}

/**
 * Build the calendar URL (modal closed).
 *
 * @example
 * buildCalendarUrl("abc-123") // "/calendar/abc-123"
 */
export function buildCalendarUrl(calendarId: string): string {
    return `/calendar/${calendarId}`
}

/**
 * Convert a ServerEntry to form data for editing.
 */
export function entryToFormData(entry: ServerEntry): EntryFormData {
    let entryType: EntryFormData["entryType"] = "all_day"

    if (entry.isTimed) {
        entryType = "timed"
    } else if (entry.isTask) {
        entryType = "task"
    } else if (entry.isMultiDay) {
        entryType = "multi_day"
    } else if (entry.isAllDay) {
        entryType = "all_day"
    }

    return {
        title: entry.title,
        date: entry.date,
        startTime: entry.startTime ?? undefined,
        endTime: entry.endTime ?? undefined,
        isAllDay: entry.isAllDay,
        description: entry.description ?? undefined,
        location: entry.location ?? undefined,
        entryType,
        endDate: entry.multiDayEndDate ?? undefined,
    }
}

/**
 * Create default form data for a new entry.
 */
export function createDefaultFormData(defaultDate?: string): EntryFormData {
    return {
        title: "",
        date: defaultDate ?? "",
        isAllDay: true,
        entryType: "all_day",
    }
}

/**
 * Convert form data to URLSearchParams for API submission.
 */
export function formDataToApiPayload(data: EntryFormData, calendarId: string): URLSearchParams {
    const params = new URLSearchParams()

    params.set("calendar_id", calendarId)
    params.set("title", data.title)
    params.set("date", data.date)

    // Determine entry_type based on isAllDay and presence of times
    if (data.isAllDay) {
        params.set("entry_type", "all_day")
    } else if (data.startTime || data.endTime) {
        params.set("entry_type", "timed")
        if (data.startTime) {
            params.set("start_time", data.startTime)
        }
        if (data.endTime) {
            params.set("end_time", data.endTime)
        }
    } else {
        params.set("entry_type", data.entryType)
    }

    if (data.description) {
        params.set("description", data.description)
    }

    if (data.location) {
        params.set("location", data.location)
    }

    if (data.endDate && data.entryType === "multi_day") {
        params.set("end_date", data.endDate)
    }

    return params
}

/**
 * Validate form data before submission.
 */
export function validateFormData(data: EntryFormData): ValidationResult {
    const errors: string[] = []

    if (!data.title.trim()) {
        errors.push("Title is required")
    }

    if (!data.date) {
        errors.push("Date is required")
    }

    // Validate date format (YYYY-MM-DD)
    if (data.date && !/^\d{4}-\d{2}-\d{2}$/.test(data.date)) {
        errors.push("Date must be in YYYY-MM-DD format")
    }

    // Validate time format if provided (HH:MM)
    if (data.startTime && !/^\d{2}:\d{2}$/.test(data.startTime)) {
        errors.push("Start time must be in HH:MM format")
    }

    if (data.endTime && !/^\d{2}:\d{2}$/.test(data.endTime)) {
        errors.push("End time must be in HH:MM format")
    }

    // Validate end time is after start time
    if (data.startTime && data.endTime && data.startTime >= data.endTime) {
        errors.push("End time must be after start time")
    }

    // Validate end date for multi-day entries
    if (data.entryType === "multi_day") {
        if (!data.endDate) {
            errors.push("End date is required for multi-day entries")
        } else if (data.date && data.endDate <= data.date) {
            errors.push("End date must be after start date")
        }
    }

    return {
        valid: errors.length === 0,
        errors,
    }
}
