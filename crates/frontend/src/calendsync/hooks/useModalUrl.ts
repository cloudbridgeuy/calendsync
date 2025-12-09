/**
 * Hook for managing URL-based modal state.
 * Imperative Shell: Handles browser history API interactions.
 */

import { buildCalendarUrl, buildModalUrl, parseModalUrl } from "@core/calendar"
import { useCallback, useEffect, useState } from "react"

import type { ModalState } from "../types"

/**
 * Configuration for useModalUrl hook.
 */
export interface UseModalUrlConfig {
  /** Calendar ID for building URLs */
  calendarId: string
  /** Initial modal state from SSR (if any) */
  initialModal?: ModalState
}

/**
 * Result from useModalUrl hook.
 */
export interface UseModalUrlResult {
  /** Current modal state (null if modal is closed) */
  modalState: ModalState | null
  /** Open the modal in create mode */
  openCreateModal: (defaultDate?: string) => void
  /** Open the modal in edit mode for a specific entry */
  openEditModal: (entryId: string) => void
  /** Close the modal via history.back() */
  closeModal: () => void
  /** Close the modal after save via history.replaceState() */
  closeAfterSave: () => void
}

/**
 * Hook to manage URL-based modal state.
 *
 * This hook:
 * - Parses the current URL to determine initial modal state
 * - Updates modal state when URL changes (popstate events)
 * - Provides functions to open/close modal via history API
 *
 * Navigation behavior:
 * - Opening modal: pushState (adds to history)
 * - Canceling: history.back() (natural back button behavior)
 * - After save: replaceState (prevents re-opening on back)
 */
export function useModalUrl(config: UseModalUrlConfig): UseModalUrlResult {
  const { calendarId, initialModal } = config

  // Initialize from SSR data or parse current URL
  const [modalState, setModalState] = useState<ModalState | null>(() => {
    if (initialModal) {
      return initialModal
    }
    // Client-side: check if we're on a modal URL
    if (typeof window !== "undefined") {
      const parsed = parseModalUrl(window.location.pathname, window.location.search)
      if (parsed) {
        return {
          mode: parsed.mode,
          entryId: parsed.entryId,
        }
      }
    }
    return null
  })

  // Listen for popstate events (back/forward navigation)
  useEffect(() => {
    const handlePopstate = () => {
      const parsed = parseModalUrl(window.location.pathname, window.location.search)
      if (parsed) {
        setModalState({
          mode: parsed.mode,
          entryId: parsed.entryId,
        })
      } else {
        setModalState(null)
      }
    }

    window.addEventListener("popstate", handlePopstate)
    return () => window.removeEventListener("popstate", handlePopstate)
  }, [])

  /**
   * Open the modal in create mode.
   */
  const openCreateModal = useCallback(
    (defaultDate?: string) => {
      const url = buildModalUrl(calendarId, "create")
      history.pushState({ modal: "create" }, "", url)
      setModalState({
        mode: "create",
        defaultDate,
      })
    },
    [calendarId],
  )

  /**
   * Open the modal in edit mode for a specific entry.
   */
  const openEditModal = useCallback(
    (entryId: string) => {
      const url = buildModalUrl(calendarId, "edit", entryId)
      history.pushState({ modal: "edit", entryId }, "", url)
      setModalState({
        mode: "edit",
        entryId,
      })
    },
    [calendarId],
  )

  /**
   * Close the modal via history.back().
   * This allows the browser back button to work naturally.
   */
  const closeModal = useCallback(() => {
    history.back()
    // Note: state update happens via popstate handler
  }, [])

  /**
   * Close the modal after save via history.replaceState().
   * This prevents the modal from re-opening when the user presses back.
   */
  const closeAfterSave = useCallback(() => {
    const url = buildCalendarUrl(calendarId)
    history.replaceState(null, "", url)
    setModalState(null)
  }, [calendarId])

  return {
    modalState,
    openCreateModal,
    openEditModal,
    closeModal,
    closeAfterSave,
  }
}
