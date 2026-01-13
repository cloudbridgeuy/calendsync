/**
 * Calendar selection logic - pure functions with no side effects.
 * This is the Functional Core for determining which calendar to display
 * and what initial view to show.
 */

/**
 * Result of calendar selection.
 * - use_stored: Use a previously stored calendar ID (user preference)
 * - use_default: Use the first available calendar (fallback)
 * - no_calendars: User has no calendars available
 */
export type CalendarSelection =
  | { type: "use_stored"; calendarId: string }
  | { type: "use_default"; calendarId: string }
  | { type: "no_calendars" }

/**
 * Selects which calendar to display based on stored preference and availability.
 *
 * Selection logic:
 * 1. If no calendars are available, return no_calendars
 * 2. If a stored calendar ID exists and is in the available list, use it
 * 3. Otherwise, use the first available calendar as default
 *
 * @param storedCalendarId - Previously stored calendar ID (or null)
 * @param availableCalendars - List of calendars the user has access to
 * @returns The calendar selection result
 */
export function selectCalendar(
  storedCalendarId: string | null,
  availableCalendars: Array<{ id: string }>,
): CalendarSelection {
  if (availableCalendars.length === 0) {
    return { type: "no_calendars" }
  }

  if (storedCalendarId && availableCalendars.some((c) => c.id === storedCalendarId)) {
    return { type: "use_stored", calendarId: storedCalendarId }
  }

  return { type: "use_default", calendarId: availableCalendars[0].id }
}

/**
 * Initial view to display when the app loads.
 * - login: Show the login screen (no valid session)
 * - loading_calendar: Show the calendar loading state (valid session)
 */
export type InitialView = "login" | "loading_calendar"

/**
 * Determines the initial view based on session validity.
 *
 * @param sessionId - The session ID (or null if not logged in)
 * @param isValid - Whether the session is valid (verified with server)
 * @returns The initial view to display
 */
export function determineInitialView(sessionId: string | null, isValid: boolean): InitialView {
  return sessionId && isValid ? "loading_calendar" : "login"
}
