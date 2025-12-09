/**
 * NotificationContext - provides shared state to NotificationCenter sub-components.
 * Enables the compound component pattern for NotificationCenter.
 */

import { buildAriaIds } from "@core/calendar"
import { createContext, useCallback, useContext, useId, useMemo } from "react"
import type { NotificationCenterActions, NotificationCenterState } from "../hooks"

/** Context value shared with notification sub-components */
export interface NotificationContextValue {
  /** Current state (notifications, isOpen, unreadCount) */
  state: NotificationCenterState
  /** Available actions (markAsRead, clearAll, etc.) */
  actions: NotificationCenterActions
  /** ARIA ID for the bell trigger button */
  triggerId: string
  /** ARIA ID for the notification panel */
  contentId: string
  /** Ref callback for the panel element */
  panelRef: React.RefCallback<HTMLDivElement>
  /** Ref callback for the bell button element */
  buttonRef: React.RefCallback<HTMLButtonElement>
}

/** NotificationContext - null when not inside provider */
const NotificationContext = createContext<NotificationContextValue | null>(null)

/** Props for NotificationCenterProvider */
export interface NotificationCenterProviderProps {
  children: React.ReactNode
  state: NotificationCenterState
  actions: NotificationCenterActions
}

/**
 * NotificationCenterProvider - wraps notification sub-components with shared context.
 */
export function NotificationCenterProvider({
  children,
  state,
  actions,
}: NotificationCenterProviderProps) {
  const id = useId()
  const { triggerId, contentId } = buildAriaIds(`notification-center-${id}`)

  // Ref callbacks - these will be used by sub-components to attach refs
  // The actual DOM elements will be accessed via document.getElementById in sub-components
  const panelRef = useCallback((_node: HTMLDivElement | null) => {
    // Callback provided for future use (e.g., click-outside detection in sub-components)
  }, [])

  const buttonRef = useCallback((_node: HTMLButtonElement | null) => {
    // Callback provided for future use (e.g., click-outside detection in sub-components)
  }, [])

  const value = useMemo<NotificationContextValue>(
    () => ({
      state,
      actions,
      triggerId,
      contentId,
      panelRef,
      buttonRef,
    }),
    [state, actions, triggerId, contentId, panelRef, buttonRef],
  )

  return <NotificationContext.Provider value={value}>{children}</NotificationContext.Provider>
}

/**
 * Hook to access NotificationContext.
 * Throws if used outside NotificationCenterProvider.
 */
export function useNotificationContext(): NotificationContextValue {
  const ctx = useContext(NotificationContext)
  if (!ctx) {
    throw new Error("useNotificationContext must be used within NotificationCenterProvider")
  }
  return ctx
}
