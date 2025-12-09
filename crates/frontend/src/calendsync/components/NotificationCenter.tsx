/**
 * NotificationCenter compound component - displays notification bell with badge and dropdown panel.
 * Uses context to share state between Bell, Panel, and Item sub-components.
 */

import { formatNotificationTime, getNotificationLabel } from "@core/calendar"
import { useCallback, useEffect } from "react"
import { NotificationCenterProvider, useNotificationContext } from "../contexts/NotificationContext"
import type { NotificationCenterActions, NotificationCenterState } from "../hooks"
import type { Notification, NotificationType } from "../types"

/**
 * Get SVG icon for notification type.
 * These are decorative icons - the notification type is already conveyed via the label text.
 */
function NotificationIcon({ type }: { type: NotificationType }) {
  switch (type) {
    case "added":
      return (
        <svg
          width={12}
          height={12}
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth={2.5}
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <title>Added</title>
          <line x1="12" y1="5" x2="12" y2="19" />
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      )
    case "updated":
      return (
        <svg
          width={12}
          height={12}
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth={2.5}
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <title>Updated</title>
          <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
        </svg>
      )
    case "deleted":
      return (
        <svg
          width={12}
          height={12}
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth={2.5}
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <title>Deleted</title>
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      )
  }
}

// ============================================================================
// Bell Sub-Component
// ============================================================================

/**
 * NotificationCenter.Bell - the notification bell button with badge.
 */
function Bell() {
  const { state, actions, triggerId, contentId, buttonRef } = useNotificationContext()
  const { isOpen, unreadCount } = state
  const { toggleOpen } = actions

  return (
    <button
      ref={buttonRef}
      id={triggerId}
      type="button"
      className={`notification-bell${isOpen ? " active" : ""}`}
      onClick={toggleOpen}
      aria-label={`Notifications${unreadCount > 0 ? ` (${unreadCount} unread)` : ""}`}
      aria-expanded={isOpen}
      aria-controls={contentId}
      aria-haspopup="dialog"
    >
      <span className="notification-bell-icon">
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
          <path d="M13.73 21a2 2 0 0 1-3.46 0" />
        </svg>
      </span>
      {unreadCount > 0 && (
        <span className="notification-badge">{unreadCount > 99 ? "99+" : unreadCount}</span>
      )}
    </button>
  )
}

// ============================================================================
// Panel Sub-Component
// ============================================================================

interface PanelProps {
  children: React.ReactNode
}

/**
 * NotificationCenter.Panel - the dropdown panel containing notifications.
 */
function Panel({ children }: PanelProps) {
  const { state, actions, isMobile, triggerId, contentId, panelRef } = useNotificationContext()
  const { isOpen, notifications, unreadCount } = state
  const { close, markAllAsRead, clearAll } = actions

  // Close on Escape key
  useEffect(() => {
    if (!isOpen) return

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        close()
      }
    }

    document.addEventListener("keydown", handleEscape)
    return () => document.removeEventListener("keydown", handleEscape)
  }, [isOpen, close])

  // Close on click outside
  useEffect(() => {
    if (!isOpen) return

    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as Node
      const panel = document.getElementById(contentId)
      const button = document.getElementById(triggerId)

      if (panel && !panel.contains(target) && button && !button.contains(target)) {
        close()
      }
    }

    document.addEventListener("mousedown", handleClickOutside)
    return () => document.removeEventListener("mousedown", handleClickOutside)
  }, [isOpen, close, contentId, triggerId])

  if (!isOpen) return null

  return (
    <>
      {/* Backdrop for mobile - purely decorative overlay */}
      {isMobile && <div className="notification-backdrop" onClick={close} aria-hidden="true" />}

      <div
        ref={panelRef}
        id={contentId}
        className={`notification-panel${isMobile ? " mobile" : ""}`}
        role="dialog"
        aria-labelledby={triggerId}
        aria-label="Notifications"
      >
        {/* Panel header */}
        <div className="notification-panel-header">
          <h2 className="notification-panel-title">Notifications</h2>
          <div className="notification-panel-actions">
            {unreadCount > 0 && (
              <button type="button" className="notification-action-btn" onClick={markAllAsRead}>
                Mark all read
              </button>
            )}
            {notifications.length > 0 && (
              <button type="button" className="notification-action-btn" onClick={clearAll}>
                Clear all
              </button>
            )}
          </div>
        </div>

        {/* Notification list */}
        <div className="notification-list">{children}</div>
      </div>
    </>
  )
}

// ============================================================================
// Item Sub-Component
// ============================================================================

interface ItemProps {
  notification: Notification
}

/**
 * NotificationCenter.Item - a single notification in the list.
 */
function Item({ notification }: ItemProps) {
  const { actions } = useNotificationContext()
  const { markAsRead, clearNotification } = actions

  const { type, entryTitle, date, timestamp, read } = notification
  const label = getNotificationLabel(type)
  const timeAgo = formatNotificationTime(timestamp)

  const handleClick = useCallback(() => {
    if (!read) {
      markAsRead(notification.id)
    }
  }, [read, markAsRead, notification.id])

  const handleDismiss = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation()
      clearNotification(notification.id)
    },
    [clearNotification, notification.id],
  )

  return (
    // biome-ignore lint/a11y/useSemanticElements: Using div with role="button" because we need a nested dismiss button, and HTML doesn't allow nested <button> elements
    <div
      className={`notification-item notification-item-${type}${read ? " read" : ""}`}
      onClick={handleClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          handleClick()
        }
      }}
    >
      <span className={`notification-item-icon notification-item-icon-${type}`}>
        <NotificationIcon type={type} />
      </span>
      <div className="notification-item-content">
        <div className="notification-item-header">
          <span className="notification-item-label">{label}</span>
          <span className="notification-item-time">{timeAgo}</span>
        </div>
        <div className="notification-item-title">{entryTitle}</div>
        <div className="notification-item-date">{date}</div>
      </div>
      <button
        type="button"
        className="notification-item-dismiss"
        onClick={handleDismiss}
        aria-label="Dismiss notification"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>
  )
}

// ============================================================================
// EmptyState Sub-Component
// ============================================================================

/**
 * NotificationCenter.EmptyState - shown when there are no notifications.
 * Checks notifications internally and returns null if there are any.
 */
function EmptyState() {
  const { state } = useNotificationContext()

  // Return null if there are notifications
  if (state.notifications.length > 0) {
    return null
  }

  return (
    <div className="notification-empty">
      <span className="notification-empty-icon">
        <svg
          width="48"
          height="48"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
          <path d="M13.73 21a2 2 0 0 1-3.46 0" />
        </svg>
      </span>
      <p>No notifications</p>
    </div>
  )
}

// ============================================================================
// Items Sub-Component
// ============================================================================

/**
 * NotificationCenter.Items - renders all notifications or empty state.
 * Handles iteration internally to encapsulate filtering logic.
 */
function Items() {
  const { state } = useNotificationContext()
  const { notifications } = state

  if (notifications.length === 0) {
    return <EmptyState />
  }

  return (
    <>
      {notifications.map((notification) => (
        <Item key={notification.id} notification={notification} />
      ))}
    </>
  )
}

// ============================================================================
// Main Component + Compound Export
// ============================================================================

interface NotificationCenterProps {
  /** Notification center state */
  state: NotificationCenterState
  /** Notification center actions */
  actions: NotificationCenterActions
  /** Whether the viewport is mobile */
  isMobile: boolean
}

/**
 * NotificationCenter compound component.
 *
 * @example
 * <NotificationCenter state={state} actions={actions} isMobile={isMobile}>
 *     <NotificationCenter.Bell />
 *     <NotificationCenter.Panel>
 *         <NotificationCenter.Items />
 *     </NotificationCenter.Panel>
 * </NotificationCenter>
 */
function NotificationCenterRoot({
  state,
  actions,
  isMobile,
  children,
}: NotificationCenterProps & { children: React.ReactNode }) {
  return (
    <NotificationCenterProvider state={state} actions={actions} isMobile={isMobile}>
      <div className="notification-center">{children}</div>
    </NotificationCenterProvider>
  )
}

// Attach sub-components as static properties
export const NotificationCenter = Object.assign(NotificationCenterRoot, {
  Bell,
  Panel,
  Items,
  Item,
  EmptyState,
})
