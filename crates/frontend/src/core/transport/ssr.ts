/**
 * SSR stub transport.
 *
 * Provides a no-op Transport implementation for server-side rendering.
 * All methods throw errors if called since SSR should not make HTTP requests.
 */

import type { Transport } from "./types"

const NOT_AVAILABLE = "Transport not available during SSR"

export function createSsrTransport(): Transport {
  return {
    async exchangeAuthCode() {
      throw new Error(NOT_AVAILABLE)
    },
    async validateSession() {
      throw new Error(NOT_AVAILABLE)
    },
    async logout() {
      throw new Error(NOT_AVAILABLE)
    },
    async fetchMyCalendars() {
      throw new Error(NOT_AVAILABLE)
    },
    async fetchEntries() {
      throw new Error(NOT_AVAILABLE)
    },
    async createEntry() {
      throw new Error(NOT_AVAILABLE)
    },
    async updateEntry() {
      throw new Error(NOT_AVAILABLE)
    },
    async deleteEntry() {
      throw new Error(NOT_AVAILABLE)
    },
    async toggleEntry() {
      throw new Error(NOT_AVAILABLE)
    },
    async fetchEntry() {
      throw new Error(NOT_AVAILABLE)
    },
    async getSession() {
      return null
    },
    async setSession() {},
    async clearSession() {},
    async getLastCalendar() {
      return null
    },
    async setLastCalendar() {},
    async clearLastCalendar() {},
  }
}
