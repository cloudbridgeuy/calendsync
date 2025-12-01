/**
 * API layer for calendar data fetching.
 * This is the Imperative Shell - handles all HTTP operations.
 */

import type { ServerDay } from "../core/calendar/types"

/**
 * API configuration.
 */
export interface ApiConfig {
    baseUrl: string
    calendarId: string
}

/**
 * Fetch entries for a date range from the server.
 */
export async function fetchEntries(
    config: ApiConfig,
    startDate: string,
    endDate: string,
): Promise<ServerDay[]> {
    const url = `${config.baseUrl}/api/calendars/${config.calendarId}/days?start=${startDate}&end=${endDate}`

    const response = await fetch(url, {
        headers: {
            Accept: "application/json",
        },
    })

    if (!response.ok) {
        throw new Error(`Failed to fetch entries: ${response.status}`)
    }

    return response.json()
}

/**
 * Create a new calendar entry.
 */
export async function createEntry(
    config: ApiConfig,
    entry: {
        title: string
        date: string
        startTime?: string
        endTime?: string
        isAllDay?: boolean
        description?: string
        location?: string
    },
): Promise<{ id: string }> {
    const url = `${config.baseUrl}/api/calendars/${config.calendarId}/entries`

    const response = await fetch(url, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            Accept: "application/json",
        },
        body: JSON.stringify(entry),
    })

    if (!response.ok) {
        throw new Error(`Failed to create entry: ${response.status}`)
    }

    return response.json()
}

/**
 * Update an existing calendar entry.
 */
export async function updateEntry(
    config: ApiConfig,
    entryId: string,
    entry: {
        title?: string
        date?: string
        startTime?: string
        endTime?: string
        isAllDay?: boolean
        description?: string
        location?: string
        completed?: boolean
    },
): Promise<void> {
    const url = `${config.baseUrl}/api/calendars/${config.calendarId}/entries/${entryId}`

    const response = await fetch(url, {
        method: "PATCH",
        headers: {
            "Content-Type": "application/json",
            Accept: "application/json",
        },
        body: JSON.stringify(entry),
    })

    if (!response.ok) {
        throw new Error(`Failed to update entry: ${response.status}`)
    }
}

/**
 * Delete a calendar entry.
 */
export async function deleteEntry(config: ApiConfig, entryId: string): Promise<void> {
    const url = `${config.baseUrl}/api/calendars/${config.calendarId}/entries/${entryId}`

    const response = await fetch(url, {
        method: "DELETE",
    })

    if (!response.ok) {
        throw new Error(`Failed to delete entry: ${response.status}`)
    }
}

/**
 * Toggle task completion status.
 */
export async function toggleTaskCompletion(
    config: ApiConfig,
    entryId: string,
    completed: boolean,
): Promise<void> {
    return updateEntry(config, entryId, { completed })
}
