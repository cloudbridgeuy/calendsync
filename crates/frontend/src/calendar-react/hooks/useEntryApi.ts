/**
 * Hook for entry CRUD API operations.
 * Imperative Shell: Handles fetch operations for entries.
 */

import { formDataToApiPayload } from "@core/calendar"
import type { ServerEntry } from "@core/calendar/types"
import { useCallback, useMemo } from "react"

import type { EntryFormData } from "../types"
import { getControlPlaneUrl } from "./useApi"

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

    const baseUrl = useMemo(() => getControlPlaneUrl(), [])

    /**
     * Create a new entry.
     */
    const createEntry = useCallback(
        async (data: EntryFormData): Promise<ServerEntry> => {
            const payload = formDataToApiPayload(data, calendarId)

            const response = await fetch(`${baseUrl}/api/entries`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/x-www-form-urlencoded",
                },
                body: payload.toString(),
            })

            if (!response.ok) {
                const text = await response.text()
                throw new Error(`Failed to create entry: ${response.status} ${text}`)
            }

            return response.json()
        },
        [baseUrl, calendarId],
    )

    /**
     * Update an existing entry.
     */
    const updateEntry = useCallback(
        async (entryId: string, data: EntryFormData): Promise<ServerEntry> => {
            const payload = formDataToApiPayload(data, calendarId)

            const response = await fetch(`${baseUrl}/api/entries/${entryId}`, {
                method: "PUT",
                headers: {
                    "Content-Type": "application/x-www-form-urlencoded",
                },
                body: payload.toString(),
            })

            if (!response.ok) {
                const text = await response.text()
                throw new Error(`Failed to update entry: ${response.status} ${text}`)
            }

            return response.json()
        },
        [baseUrl, calendarId],
    )

    /**
     * Delete an entry.
     */
    const deleteEntry = useCallback(
        async (entryId: string): Promise<void> => {
            const response = await fetch(`${baseUrl}/api/entries/${entryId}`, {
                method: "DELETE",
            })

            if (!response.ok) {
                const text = await response.text()
                throw new Error(`Failed to delete entry: ${response.status} ${text}`)
            }
        },
        [baseUrl],
    )

    /**
     * Fetch a specific entry by ID.
     * Used when navigating directly to an edit URL on the client side.
     */
    const fetchEntry = useCallback(
        async (entryId: string): Promise<ServerEntry> => {
            const response = await fetch(`${baseUrl}/api/entries/${entryId}`)

            if (!response.ok) {
                const text = await response.text()
                throw new Error(`Failed to fetch entry: ${response.status} ${text}`)
            }

            return response.json()
        },
        [baseUrl],
    )

    return {
        createEntry,
        updateEntry,
        deleteEntry,
        fetchEntry,
    }
}
