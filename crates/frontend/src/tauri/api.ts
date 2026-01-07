/**
 * Tauri API module - wraps Tauri IPC commands and HTTP API calls.
 *
 * This module provides:
 * - Tauri command wrappers that call `invoke()` for IPC
 * - HTTP API wrappers for authenticated requests
 */

import { invoke } from "@tauri-apps/api/core"

// Types

export interface Calendar {
  id: string
  name: string
  color: string
  description?: string
}

export interface CalendarWithRole {
  id: string
  name: string
  color: string
  description?: string
  role: string
}

// Tauri command wrappers

/**
 * Get the current session ID from persistent storage.
 *
 * @returns The session ID if one exists, or `null`.
 */
export async function getSession(): Promise<string | null> {
  return invoke<string | null>("get_session")
}

/**
 * Save a session ID to persistent storage.
 *
 * @param sessionId - The session ID to save
 */
export async function setSession(sessionId: string): Promise<void> {
  return invoke("set_session", { sessionId })
}

/**
 * Clear the current session from persistent storage.
 */
export async function clearSession(): Promise<void> {
  return invoke("clear_session")
}

/**
 * Get the last-used calendar ID from persistent storage.
 *
 * @returns The calendar ID if one exists, or `null`.
 */
export async function getLastCalendar(): Promise<string | null> {
  return invoke<string | null>("get_last_calendar")
}

/**
 * Save the last-used calendar ID to persistent storage.
 *
 * @param calendarId - The calendar ID to save
 */
export async function setLastCalendar(calendarId: string): Promise<void> {
  return invoke("set_last_calendar", { calendarId })
}

/**
 * Clear the last-used calendar ID from persistent storage.
 */
export async function clearLastCalendar(): Promise<void> {
  return invoke("clear_last_calendar")
}

/**
 * Open the system browser to initiate OAuth login with the specified provider.
 *
 * @param provider - The OAuth provider name ("google" or "apple")
 */
export async function openOAuthLogin(provider: "google" | "apple"): Promise<void> {
  return invoke("open_oauth_login", { provider })
}

// HTTP API wrappers

/**
 * Exchange an authorization code for a session.
 * This sets a cookie in the webview for subsequent authenticated requests.
 *
 * @param apiUrl - The base API URL
 * @param code - The authorization code from OAuth
 * @param state - The state parameter for CSRF protection
 * @returns The session ID
 * @throws Error if exchange fails
 */
export async function exchangeCodeForSession(
  apiUrl: string,
  code: string,
  state: string,
): Promise<string> {
  const response = await fetch(`${apiUrl}/auth/exchange`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ code, state }),
    credentials: "include", // This sets the cookie in the webview
  })

  if (!response.ok) {
    const body = await response.text()
    throw new Error(`Auth exchange failed: ${response.status} ${body}`)
  }

  const data = await response.json()
  return data.session_id
}

/**
 * Fetch the list of calendars the authenticated user has access to.
 * Uses cookie-based authentication.
 *
 * @param apiUrl - The base API URL
 * @returns Array of calendars with role information
 * @throws Error with message "UNAUTHORIZED" if session is invalid
 */
export async function fetchUserCalendars(apiUrl: string): Promise<CalendarWithRole[]> {
  const response = await fetch(`${apiUrl}/api/calendars/me`, {
    credentials: "include",
  })
  if (!response.ok) {
    if (response.status === 401) {
      throw new Error("UNAUTHORIZED")
    }
    throw new Error(`Failed to fetch calendars: ${response.status}`)
  }
  return response.json()
}

/**
 * Validate a session by calling the /auth/me endpoint.
 * Uses cookie-based authentication.
 *
 * @param apiUrl - The base API URL
 * @returns `true` if session is valid, `false` otherwise
 */
export async function validateSession(apiUrl: string): Promise<boolean> {
  try {
    const response = await fetch(`${apiUrl}/auth/me`, {
      credentials: "include",
    })
    return response.ok
  } catch {
    return false
  }
}
