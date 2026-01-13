/**
 * Web transport implementation.
 *
 * Uses fetch() with cookies for browser-based HTTP communication.
 * This is the default transport for the web version of the app.
 */

import type { ServerDay, ServerEntry } from "../calendar/types"
import type { CalendarWithRole, CreateEntryPayload, FetchEntriesOptions, Transport } from "./types"

/**
 * Convert CreateEntryPayload to URLSearchParams for form submission.
 */
function payloadToFormData(payload: CreateEntryPayload): URLSearchParams {
  const params = new URLSearchParams()

  params.set("calendar_id", payload.calendar_id)
  params.set("title", payload.title)
  params.set("start_date", payload.date)

  // Determine entry_type
  if (payload.entry_type) {
    params.set("entry_type", payload.entry_type)
  } else if (payload.all_day) {
    params.set("entry_type", "all_day")
  } else if (payload.start_time || payload.end_time) {
    params.set("entry_type", "timed")
  } else {
    params.set("entry_type", "all_day")
  }

  if (payload.start_time) {
    params.set("start_time", payload.start_time)
  }
  if (payload.end_time) {
    params.set("end_time", payload.end_time)
  }
  if (payload.description) {
    params.set("description", payload.description)
  }

  return params
}

/**
 * Create a web transport that uses fetch() for HTTP communication.
 *
 * @param baseUrl - The base URL for API requests (e.g., "http://localhost:3000")
 */
export function createWebTransport(baseUrl: string): Transport {
  return {
    // Auth operations

    async exchangeAuthCode(code: string, state: string): Promise<string> {
      const response = await fetch(`${baseUrl}/auth/exchange`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ code, state }),
        credentials: "include",
      })

      if (!response.ok) {
        const body = await response.text()
        throw new Error(`Auth exchange failed: ${response.status} ${body}`)
      }

      const data = await response.json()
      return data.session_id
    },

    async validateSession(): Promise<boolean> {
      try {
        const response = await fetch(`${baseUrl}/auth/me`, {
          credentials: "include",
        })
        return response.ok
      } catch {
        return false
      }
    },

    async logout(): Promise<void> {
      await fetch(`${baseUrl}/auth/logout`, {
        method: "POST",
        credentials: "include",
      })
    },

    // Calendar operations

    async fetchMyCalendars(): Promise<CalendarWithRole[]> {
      const response = await fetch(`${baseUrl}/api/calendars/me`, {
        credentials: "include",
      })

      if (!response.ok) {
        if (response.status === 401) {
          throw new Error("UNAUTHORIZED")
        }
        throw new Error(`Failed to fetch calendars: ${response.status}`)
      }

      return response.json()
    },

    async fetchEntries(opts: FetchEntriesOptions): Promise<ServerDay[]> {
      const params = new URLSearchParams({
        calendar_id: opts.calendarId,
        highlighted_day: opts.highlightedDay,
      })

      if (opts.before !== undefined) {
        params.set("before", opts.before.toString())
      }
      if (opts.after !== undefined) {
        params.set("after", opts.after.toString())
      }

      const response = await fetch(`${baseUrl}/api/entries?${params}`, {
        signal: opts.signal,
        credentials: "include",
      })

      if (!response.ok) {
        throw new Error(`Failed to fetch entries: ${response.status}`)
      }

      return response.json()
    },

    // Entry operations

    async createEntry(payload: CreateEntryPayload): Promise<ServerEntry> {
      const formData = payloadToFormData(payload)

      const response = await fetch(`${baseUrl}/api/entries`, {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body: formData.toString(),
        credentials: "include",
      })

      if (!response.ok) {
        const text = await response.text()
        throw new Error(`Failed to create entry: ${response.status} ${text}`)
      }

      return response.json()
    },

    async updateEntry(id: string, payload: CreateEntryPayload): Promise<ServerEntry> {
      const formData = payloadToFormData(payload)

      const response = await fetch(`${baseUrl}/api/entries/${id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body: formData.toString(),
        credentials: "include",
      })

      if (!response.ok) {
        const text = await response.text()
        throw new Error(`Failed to update entry: ${response.status} ${text}`)
      }

      return response.json()
    },

    async deleteEntry(id: string): Promise<void> {
      const response = await fetch(`${baseUrl}/api/entries/${id}`, {
        method: "DELETE",
        credentials: "include",
      })

      if (!response.ok) {
        throw new Error(`Failed to delete entry: ${response.status}`)
      }
    },

    async toggleEntry(id: string): Promise<ServerEntry> {
      const response = await fetch(`${baseUrl}/api/entries/${id}/toggle`, {
        method: "PATCH",
        credentials: "include",
      })

      if (!response.ok) {
        throw new Error(`Failed to toggle entry: ${response.status}`)
      }

      return response.json()
    },

    async fetchEntry(id: string): Promise<ServerEntry> {
      const response = await fetch(`${baseUrl}/api/entries/${id}`, {
        credentials: "include",
      })

      if (!response.ok) {
        throw new Error(`Failed to fetch entry: ${response.status}`)
      }

      return response.json()
    },

    // Persistent storage - web version uses cookies managed by browser
    // These are no-ops since the browser handles session cookies automatically

    async getSession(): Promise<string | null> {
      // Session is managed via cookies, not accessible from JS
      return null
    },

    async setSession(_id: string): Promise<void> {
      // Session is set via Set-Cookie header from server
    },

    async clearSession(): Promise<void> {
      // Logout endpoint clears the cookie
      await this.logout()
    },

    async getLastCalendar(): Promise<string | null> {
      // Use localStorage for web
      if (typeof localStorage !== "undefined") {
        return localStorage.getItem("lastCalendarId")
      }
      return null
    },

    async setLastCalendar(id: string): Promise<void> {
      if (typeof localStorage !== "undefined") {
        localStorage.setItem("lastCalendarId", id)
      }
    },

    async clearLastCalendar(): Promise<void> {
      if (typeof localStorage !== "undefined") {
        localStorage.removeItem("lastCalendarId")
      }
    },
  }
}
