/**
 * Tauri transport implementation.
 *
 * Uses Tauri's invoke() to call Rust backend commands, which handle HTTP
 * communication with the server. This bypasses browser CSP/CORS restrictions.
 */

import { invoke } from "@tauri-apps/api/core"
import type { ServerDay, ServerEntry } from "../calendar/types"
import type { CalendarWithRole, Transport } from "./types"

/**
 * Create a Tauri transport that routes all HTTP through Rust backend.
 */
export function createTauriTransport(): Transport {
  return {
    async exchangeAuthCode(code, state) {
      return invoke<string>("exchange_auth_code", { code, state })
    },

    async validateSession() {
      return invoke<boolean>("validate_session")
    },

    async logout() {
      await invoke("logout")
    },

    async fetchMyCalendars() {
      return invoke<CalendarWithRole[]>("fetch_my_calendars")
    },

    async fetchEntries(opts) {
      return invoke<ServerDay[]>("fetch_entries", {
        calendarId: opts.calendarId,
        highlightedDay: opts.highlightedDay,
        before: opts.before,
        after: opts.after,
      })
    },

    async createEntry(payload) {
      return invoke<ServerEntry>("create_entry", { payload })
    },

    async updateEntry(id, payload) {
      return invoke<ServerEntry>("update_entry", { id, payload })
    },

    async deleteEntry(id) {
      await invoke("delete_entry", { id })
    },

    async toggleEntry(id) {
      return invoke<ServerEntry>("toggle_entry", { id })
    },

    // Persistent storage via Tauri store plugin
    async getSession() {
      return invoke<string | null>("get_session")
    },

    async setSession(id) {
      await invoke("set_session", { sessionId: id })
    },

    async clearSession() {
      await invoke("clear_session")
    },

    async getLastCalendar() {
      return invoke<string | null>("get_last_calendar")
    },

    async setLastCalendar(id) {
      await invoke("set_last_calendar", { calendarId: id })
    },

    async clearLastCalendar() {
      await invoke("clear_last_calendar")
    },
  }
}
