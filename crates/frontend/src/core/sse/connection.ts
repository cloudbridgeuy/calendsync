/**
 * Pure functions for SSE connection management.
 * This is the Functional Core for SSE connection logic.
 *
 * All functions are pure (no side effects) and can be tested in isolation.
 */

/** Default reconnection delay in milliseconds */
export const RECONNECT_DELAY_MS = 3000

/** Maximum reconnection attempts before giving up */
export const MAX_RECONNECT_ATTEMPTS = 5

/**
 * Calculate reconnection delay with exponential backoff.
 *
 * @param attempts - Number of reconnection attempts so far
 * @param baseDelay - Base delay in milliseconds (default: RECONNECT_DELAY_MS)
 * @param maxExponent - Maximum exponent for backoff (default: 4, caps at 16x base delay)
 * @returns Delay in milliseconds before next reconnection attempt
 */
export function calculateReconnectDelay(
  attempts: number,
  baseDelay: number = RECONNECT_DELAY_MS,
  maxExponent: number = 4,
): number {
  const exponent = Math.min(attempts, maxExponent)
  return baseDelay * 2 ** exponent
}

/**
 * Determine if another reconnection attempt should be made.
 *
 * @param attempts - Number of reconnection attempts so far
 * @param maxAttempts - Maximum allowed attempts (default: MAX_RECONNECT_ATTEMPTS)
 * @returns True if another attempt should be made
 */
export function shouldReconnect(
  attempts: number,
  maxAttempts: number = MAX_RECONNECT_ATTEMPTS,
): boolean {
  return attempts < maxAttempts
}

/**
 * Safely parse JSON event data from SSE.
 *
 * @param data - Raw JSON string from SSE event
 * @returns Parsed object or null if parsing fails
 */
export function parseEventData<T>(data: string): T | null {
  try {
    return JSON.parse(data) as T
  } catch {
    return null
  }
}

/**
 * Build SSE URL with optional last event ID for reconnection.
 *
 * @param baseUrl - Base URL for the SSE endpoint
 * @param calendarId - Calendar ID to subscribe to
 * @param lastEventId - Optional last event ID for resuming
 * @returns Complete SSE URL
 */
export function buildSseUrl(
  baseUrl: string,
  calendarId: string,
  lastEventId: string | null = null,
): string {
  let url = `${baseUrl}/api/events?calendar_id=${encodeURIComponent(calendarId)}`
  if (lastEventId) {
    url += `&last_event_id=${encodeURIComponent(lastEventId)}`
  }
  return url
}
