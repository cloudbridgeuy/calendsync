/**
 * Type definitions for Tauri app authentication state machine.
 */

/**
 * Application view states.
 *
 * - `loading`: Initial state while checking session
 * - `login`: No valid session, show login screen
 * - `calendar`: Authenticated, show calendar view
 */
export type AppView = "loading" | "login" | "calendar"

/**
 * Authentication state for the Tauri app.
 *
 * Tracks the current view, session, calendar, and any errors.
 */
export interface AuthState {
  /** Current view being displayed */
  view: AppView
  /** Session ID if authenticated, null otherwise */
  sessionId: string | null
  /** Calendar ID to display, null if none selected */
  calendarId: string | null
  /** Error message if something went wrong */
  error: string | null
}
