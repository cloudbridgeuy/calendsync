/**
 * Notification center hook - Imperative Shell.
 * Handles localStorage persistence and provides notification state management.
 */

import {
  addNotification as addNotificationPure,
  countUnread,
  createNotification,
  getStorageKey,
  markAllNotificationsAsRead as markAllPure,
  markNotificationAsRead as markAsReadPure,
  parseNotificationsJson,
  removeNotification as removePure,
  serializeNotifications,
} from "@core/calendar/notifications"
import { useCallback, useEffect, useMemo, useState } from "react"
import type { Notification, NotificationType } from "../types"

/** Configuration for useNotificationCenter hook */
export interface UseNotificationCenterConfig {
  /** Calendar ID for localStorage key */
  calendarId: string
}

/** State returned by useNotificationCenter */
export interface NotificationCenterState {
  /** List of notifications (newest first) */
  notifications: Notification[]
  /** Whether the notification panel is open */
  isOpen: boolean
  /** Count of unread notifications */
  unreadCount: number
}

/** Actions returned by useNotificationCenter */
export interface NotificationCenterActions {
  /** Mark a notification as read */
  markAsRead: (id: string) => void
  /** Mark all notifications as read */
  markAllAsRead: () => void
  /** Clear all notifications */
  clearAll: () => void
  /** Remove a single notification */
  clearNotification: (id: string) => void
  /** Toggle the notification panel open/closed */
  toggleOpen: () => void
  /** Close the notification panel */
  close: () => void
}

/** Function to add a new notification */
export type AddNotificationFn = (
  type: NotificationType,
  entryId: string,
  entryTitle: string,
  date: string,
) => void

/**
 * Hook to manage notification center state with localStorage persistence.
 *
 * @param config - Hook configuration
 * @returns Tuple of [state, actions, addNotification]
 */
export function useNotificationCenter(
  config: UseNotificationCenterConfig,
): [NotificationCenterState, NotificationCenterActions, AddNotificationFn] {
  const { calendarId } = config
  const [notifications, setNotifications] = useState<Notification[]>([])
  const [isOpen, setIsOpen] = useState(false)

  // Load from localStorage on mount
  useEffect(() => {
    if (typeof window === "undefined") return
    const storageKey = getStorageKey(calendarId)
    const stored = localStorage.getItem(storageKey)
    const parsed = parseNotificationsJson(stored)
    setNotifications(parsed)
  }, [calendarId])

  // Persist to localStorage on change
  useEffect(() => {
    if (typeof window === "undefined") return
    // Skip if notifications are empty on initial render
    // (we only want to save after user interaction or new notifications)
    const storageKey = getStorageKey(calendarId)
    const serialized = serializeNotifications(notifications)
    localStorage.setItem(storageKey, serialized)
  }, [calendarId, notifications])

  const unreadCount = useMemo(() => countUnread(notifications), [notifications])

  const addNotification = useCallback<AddNotificationFn>((type, entryId, entryTitle, date) => {
    const notification = createNotification(type, entryId, entryTitle, date)
    setNotifications((prev) => addNotificationPure(prev, notification))
  }, [])

  const markAsRead = useCallback((id: string) => {
    setNotifications((prev) => markAsReadPure(prev, id))
  }, [])

  const markAllAsRead = useCallback(() => {
    setNotifications((prev) => markAllPure(prev))
  }, [])

  const clearAll = useCallback(() => {
    setNotifications([])
  }, [])

  const clearNotification = useCallback((id: string) => {
    setNotifications((prev) => removePure(prev, id))
  }, [])

  const toggleOpen = useCallback(() => {
    setIsOpen((prev) => !prev)
  }, [])

  const close = useCallback(() => {
    setIsOpen(false)
  }, [])

  const state: NotificationCenterState = {
    notifications,
    isOpen,
    unreadCount,
  }

  const actions: NotificationCenterActions = {
    markAsRead,
    markAllAsRead,
    clearAll,
    clearNotification,
    toggleOpen,
    close,
  }

  return [state, actions, addNotification]
}
