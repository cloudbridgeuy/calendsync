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

export interface FetchEntriesOptions {
  calendarId: string
  highlightedDay: string
  /** Days before highlightedDay (server defaults to 365 if omitted) */
  before?: number
  /** Days after highlightedDay (server defaults to 365 if omitted) */
  after?: number
  /** Optional auth token for authenticated requests (Bearer header) */
  authToken?: string
  /** Use credentials (cookies) for authentication instead of authToken */
  useCredentials?: boolean
  /** Optional AbortSignal for cancellation */
  signal?: AbortSignal
}

/**
 * Fetch entries from the API for a date range.
 * Uses server defaults for date range (365 days before/after) if not specified.
 *
 * @returns Promise resolving to array of ServerDay objects
 */
export async function fetchEntries(options: FetchEntriesOptions): Promise<ServerDay[]> {
  const { calendarId, highlightedDay, before, after, authToken, useCredentials, signal } = options

  const params = new URLSearchParams({
    calendar_id: calendarId,
    highlighted_day: highlightedDay,
  })

  if (before !== undefined) {
    params.set("before", before.toString())
  }
  if (after !== undefined) {
    params.set("after", after.toString())
  }

  const url = `${CONTROL_PLANE_URL}/api/entries?${params.toString()}`

  const headers: HeadersInit = {}
  if (authToken && !useCredentials) {
    headers.Authorization = `Bearer ${authToken}`
  }

  const response = await fetch(url, {
    headers,
    signal,
    credentials: useCredentials ? "include" : "same-origin",
  })

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
