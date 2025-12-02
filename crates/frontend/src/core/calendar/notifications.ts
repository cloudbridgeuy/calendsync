/**
 * Pure notification functions - Functional Core.
 * No side effects (localStorage operations happen in the imperative shell).
 */

import type { Notification, NotificationType } from "../../calendar-react/types"

/** localStorage key prefix for notifications */
export const NOTIFICATION_STORAGE_PREFIX = "calendsync_notifications_"

/** Maximum number of notifications to keep */
export const MAX_NOTIFICATIONS = 50

/**
 * Create a new notification.
 */
export function createNotification(
    type: NotificationType,
    entryId: string,
    entryTitle: string,
    date: string,
): Notification {
    return {
        id: `notif-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`,
        type,
        entryId,
        entryTitle,
        date,
        timestamp: Date.now(),
        read: false,
    }
}

/**
 * Add a notification to the list, maintaining max count.
 * Returns a new array (immutable).
 */
export function addNotification(
    notifications: Notification[],
    notification: Notification,
    maxCount: number = MAX_NOTIFICATIONS,
): Notification[] {
    return [notification, ...notifications].slice(0, maxCount)
}

/**
 * Mark a notification as read.
 * Returns a new array (immutable).
 */
export function markNotificationAsRead(notifications: Notification[], id: string): Notification[] {
    return notifications.map((n) => (n.id === id ? { ...n, read: true } : n))
}

/**
 * Mark all notifications as read.
 * Returns a new array (immutable).
 */
export function markAllNotificationsAsRead(notifications: Notification[]): Notification[] {
    return notifications.map((n) => ({ ...n, read: true }))
}

/**
 * Remove a notification by ID.
 * Returns a new array (immutable).
 */
export function removeNotification(notifications: Notification[], id: string): Notification[] {
    return notifications.filter((n) => n.id !== id)
}

/**
 * Count unread notifications.
 */
export function countUnread(notifications: Notification[]): number {
    return notifications.filter((n) => !n.read).length
}

/**
 * Get the localStorage key for a calendar's notifications.
 */
export function getStorageKey(calendarId: string): string {
    return `${NOTIFICATION_STORAGE_PREFIX}${calendarId}`
}

/**
 * Parse notifications from JSON string.
 * Returns empty array for invalid/null input.
 */
export function parseNotificationsJson(json: string | null): Notification[] {
    if (!json) return []
    try {
        const parsed = JSON.parse(json)
        if (!Array.isArray(parsed)) return []
        return parsed.filter(
            (n): n is Notification =>
                typeof n === "object" &&
                n !== null &&
                typeof n.id === "string" &&
                typeof n.type === "string" &&
                (n.type === "added" || n.type === "updated" || n.type === "deleted") &&
                typeof n.entryId === "string" &&
                typeof n.entryTitle === "string" &&
                typeof n.date === "string" &&
                typeof n.timestamp === "number" &&
                typeof n.read === "boolean",
        )
    } catch {
        return []
    }
}

/**
 * Serialize notifications to JSON string.
 */
export function serializeNotifications(notifications: Notification[]): string {
    return JSON.stringify(notifications)
}

/**
 * Format a timestamp as a relative time string.
 * e.g., "just now", "2m ago", "1h ago", "3d ago"
 */
export function formatNotificationTime(timestamp: number): string {
    const now = Date.now()
    const diff = now - timestamp
    const seconds = Math.floor(diff / 1000)
    const minutes = Math.floor(seconds / 60)
    const hours = Math.floor(minutes / 60)
    const days = Math.floor(hours / 24)

    if (seconds < 60) return "just now"
    if (minutes < 60) return `${minutes}m ago`
    if (hours < 24) return `${hours}h ago`
    if (days < 7) return `${days}d ago`
    return new Date(timestamp).toLocaleDateString()
}

/**
 * Get the icon for a notification type.
 */
export function getNotificationIcon(type: NotificationType): string {
    switch (type) {
        case "added":
            return "+"
        case "updated":
            return "~"
        case "deleted":
            return "-"
    }
}

/**
 * Get a human-readable label for a notification type.
 */
export function getNotificationLabel(type: NotificationType): string {
    switch (type) {
        case "added":
            return "Added"
        case "updated":
            return "Updated"
        case "deleted":
            return "Deleted"
    }
}
