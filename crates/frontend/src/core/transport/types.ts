/**
 * Transport layer types for cross-platform HTTP abstraction.
 *
 * This module defines the Transport interface that abstracts HTTP communication,
 * allowing web and desktop (Tauri) platforms to share the same hooks and components
 * while using different underlying implementations.
 */

import type { ServerDay, ServerEntry } from "../calendar/types"

/**
 * Calendar with user's role/permission level.
 */
export interface CalendarWithRole {
  id: string
  name: string
  color: string
  description?: string
  role: string
}

/**
 * Options for fetching calendar entries.
 */
export interface FetchEntriesOptions {
  calendarId: string
  highlightedDay: string
  before?: number
  after?: number
  signal?: AbortSignal
}

/**
 * Payload for creating or updating an entry.
 */
export interface CreateEntryPayload {
  calendar_id: string
  title: string
  date: string
  start_time?: string
  end_time?: string
  all_day?: boolean
  description?: string
  entry_type?: string
}

/**
 * Transport interface for HTTP operations.
 *
 * Implementations:
 * - WebTransport: Uses fetch() with cookies for browser
 * - TauriTransport: Uses invoke() to call Rust backend
 */
export interface Transport {
  // Auth operations
  exchangeAuthCode(code: string, state: string): Promise<string>
  validateSession(): Promise<boolean>
  logout(): Promise<void>

  // Calendar operations
  fetchMyCalendars(): Promise<CalendarWithRole[]>
  fetchEntries(opts: FetchEntriesOptions): Promise<ServerDay[]>

  // Entry operations
  createEntry(payload: CreateEntryPayload): Promise<ServerEntry>
  updateEntry(id: string, payload: CreateEntryPayload): Promise<ServerEntry>
  deleteEntry(id: string): Promise<void>
  toggleEntry(id: string): Promise<ServerEntry>

  // Persistent storage (session, last calendar)
  getSession(): Promise<string | null>
  setSession(id: string): Promise<void>
  clearSession(): Promise<void>
  getLastCalendar(): Promise<string | null>
  setLastCalendar(id: string): Promise<void>
  clearLastCalendar(): Promise<void>
}
