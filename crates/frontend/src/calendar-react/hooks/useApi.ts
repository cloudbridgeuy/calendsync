/**
 * API hook for fetching calendar entries.
 */

import type { ServerDay } from "@core/calendar/types"

// CONTROL_PLANE_URL is set from __INITIAL_DATA__ at hydration time
let CONTROL_PLANE_URL = ""

/**
 * Initialize the control plane URL from initial data.
 * This should be called once during hydration.
 */
export function initControlPlaneUrl(url: string): void {
    CONTROL_PLANE_URL = url
}

/**
 * Fetch entries from the API for a date range.
 *
 * @param calendarId - The calendar ID
 * @param highlightedDay - The center date (ISO 8601: YYYY-MM-DD)
 * @param before - Number of days before highlightedDay
 * @param after - Number of days after highlightedDay
 * @param signal - Optional AbortSignal for cancellation
 * @returns Promise resolving to array of ServerDay objects
 */
export async function fetchEntries(
    calendarId: string,
    highlightedDay: string,
    before: number = 3,
    after: number = 3,
    signal?: AbortSignal,
): Promise<ServerDay[]> {
    const params = new URLSearchParams({
        calendar_id: calendarId,
        highlighted_day: highlightedDay,
        before: before.toString(),
        after: after.toString(),
    })

    const url = `${CONTROL_PLANE_URL}/api/entries/calendar?${params.toString()}`

    const response = await fetch(url, { signal })

    if (!response.ok) {
        throw new Error(`Failed to fetch entries: ${response.status} ${response.statusText}`)
    }

    return response.json()
}

/**
 * Hook configuration for API calls
 */
export interface UseApiConfig {
    calendarId: string
    controlPlaneUrl?: string
}

/**
 * Get the control plane URL (for use in SSR context where process.env might not be available)
 */
export function getControlPlaneUrl(): string {
    return CONTROL_PLANE_URL
}
