/**
 * Hook for entry CRUD API operations.
 * Imperative Shell: Handles fetch operations for entries via transport layer.
 */

import type { ServerEntry } from "@core/calendar/types"
import { useTransport } from "@core/transport"
import type { CreateEntryPayload } from "@core/transport/types"
import { useCallback } from "react"

import type { EntryFormData } from "../types"

/**
 * Configuration for useEntryApi hook.
 */
export interface UseEntryApiConfig {
  /** Calendar ID for API calls */
  calendarId: string
}

/**
 * Result from useEntryApi hook.
 */
export interface UseEntryApiResult {
  /** Create a new entry */
  createEntry: (data: EntryFormData) => Promise<ServerEntry>
  /** Update an existing entry */
  updateEntry: (entryId: string, data: EntryFormData) => Promise<ServerEntry>
  /** Delete an entry */
  deleteEntry: (entryId: string) => Promise<void>
  /** Fetch a specific entry by ID */
  fetchEntry: (entryId: string) => Promise<ServerEntry>
  /** Toggle a task's completed status */
  toggleEntry: (entryId: string) => Promise<ServerEntry>
}

/**
 * Convert EntryFormData to CreateEntryPayload for transport.
 */
function formDataToPayload(data: EntryFormData, calendarId: string): CreateEntryPayload {
  return {
    calendar_id: calendarId,
    title: data.title,
    date: data.startDate,
    start_time: data.startTime,
    end_time: data.endTime,
    all_day: data.isAllDay,
    description: data.description,
    entry_type: data.entryType,
  }
}

/**
 * Hook for entry CRUD API operations.
 *
 * This hook provides functions to:
 * - Create new entries (POST /api/entries)
 * - Update existing entries (PUT /api/entries/{id})
 * - Delete entries (DELETE /api/entries/{id})
 * - Fetch individual entries (GET /api/entries/{id})
 */
export function useEntryApi(config: UseEntryApiConfig): UseEntryApiResult {
  const { calendarId } = config
  const transport = useTransport()

  /**
   * Create a new entry.
   */
  const createEntry = useCallback(
    async (data: EntryFormData): Promise<ServerEntry> => {
      const payload = formDataToPayload(data, calendarId)
      return transport.createEntry(payload)
    },
    [transport, calendarId],
  )

  /**
   * Update an existing entry.
   */
  const updateEntry = useCallback(
    async (entryId: string, data: EntryFormData): Promise<ServerEntry> => {
      const payload = formDataToPayload(data, calendarId)
      return transport.updateEntry(entryId, payload)
    },
    [transport, calendarId],
  )

  /**
   * Delete an entry.
   */
  const deleteEntry = useCallback(
    async (entryId: string): Promise<void> => {
      return transport.deleteEntry(entryId)
    },
    [transport],
  )

  /**
   * Fetch a specific entry by ID.
   * Used when navigating directly to an edit URL on the client side.
   */
  const fetchEntry = useCallback(
    async (entryId: string): Promise<ServerEntry> => {
      return transport.fetchEntry(entryId)
    },
    [transport],
  )

  /**
   * Toggle a task's completed status.
   */
  const toggleEntry = useCallback(
    async (entryId: string): Promise<ServerEntry> => {
      return transport.toggleEntry(entryId)
    },
    [transport],
  )

  return {
    createEntry,
    updateEntry,
    deleteEntry,
    fetchEntry,
    toggleEntry,
  }
}
