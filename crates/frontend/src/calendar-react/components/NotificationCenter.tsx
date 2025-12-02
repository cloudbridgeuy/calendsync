/**
 * Notification Center component - displays notification bell with badge and dropdown panel.
 */

import { formatNotificationTime, getNotificationIcon, getNotificationLabel } from "@core/calendar"
import { useCallback, useEffect, useRef } from "react"
import type { NotificationCenterActions, NotificationCenterState } from "../hooks"
import type { Notification } from "../types"

interface NotificationCenterProps {
    /** Notification center state */
    state: NotificationCenterState
    /** Notification center actions */
    actions: NotificationCenterActions
    /** Whether the viewport is mobile */
    isMobile: boolean
}

/**
 * Notification Center with bell icon, badge, and dropdown panel.
 */
export function NotificationCenter({ state, actions, isMobile }: NotificationCenterProps) {
    const { notifications, isOpen, unreadCount } = state
    const { toggleOpen, close, markAsRead, markAllAsRead, clearAll, clearNotification } = actions
    const panelRef = useRef<HTMLDivElement>(null)
    const buttonRef = useRef<HTMLButtonElement>(null)

    // Close on click outside
    useEffect(() => {
        if (!isOpen) return

        const handleClickOutside = (e: MouseEvent) => {
            const target = e.target as Node
            if (
                panelRef.current &&
                !panelRef.current.contains(target) &&
                buttonRef.current &&
                !buttonRef.current.contains(target)
            ) {
                close()
            }
        }

        document.addEventListener("mousedown", handleClickOutside)
        return () => document.removeEventListener("mousedown", handleClickOutside)
    }, [isOpen, close])

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

    const handleNotificationClick = useCallback(
        (notification: Notification) => {
            if (!notification.read) {
                markAsRead(notification.id)
            }
        },
        [markAsRead],
    )

    const handleDismiss = useCallback(
        (e: React.MouseEvent, id: string) => {
            e.stopPropagation()
            clearNotification(id)
        },
        [clearNotification],
    )

    return (
        <div className="notification-center">
            {/* Bell button */}
            <button
                ref={buttonRef}
                type="button"
                className={`notification-bell${isOpen ? " active" : ""}`}
                onClick={toggleOpen}
                aria-label={`Notifications${unreadCount > 0 ? ` (${unreadCount} unread)` : ""}`}
                aria-expanded={isOpen}
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
                    <span className="notification-badge">
                        {unreadCount > 99 ? "99+" : unreadCount}
                    </span>
                )}
            </button>

            {/* Notification panel */}
            {isOpen && (
                <>
                    {/* Backdrop for mobile - purely decorative overlay */}
                    {isMobile && (
                        <div className="notification-backdrop" onClick={close} aria-hidden="true" />
                    )}

                    <div
                        ref={panelRef}
                        className={`notification-panel${isMobile ? " mobile" : ""}`}
                        role="dialog"
                        aria-label="Notifications"
                    >
                        {/* Panel header */}
                        <div className="notification-panel-header">
                            <h2 className="notification-panel-title">Notifications</h2>
                            <div className="notification-panel-actions">
                                {unreadCount > 0 && (
                                    <button
                                        type="button"
                                        className="notification-action-btn"
                                        onClick={markAllAsRead}
                                    >
                                        Mark all read
                                    </button>
                                )}
                                {notifications.length > 0 && (
                                    <button
                                        type="button"
                                        className="notification-action-btn"
                                        onClick={clearAll}
                                    >
                                        Clear all
                                    </button>
                                )}
                            </div>
                        </div>

                        {/* Notification list */}
                        <div className="notification-list">
                            {notifications.length === 0 ? (
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
                            ) : (
                                notifications.map((notification) => (
                                    <NotificationItem
                                        key={notification.id}
                                        notification={notification}
                                        onClick={() => handleNotificationClick(notification)}
                                        onDismiss={(e) => handleDismiss(e, notification.id)}
                                    />
                                ))
                            )}
                        </div>
                    </div>
                </>
            )}
        </div>
    )
}

interface NotificationItemProps {
    notification: Notification
    onClick: () => void
    onDismiss: (e: React.MouseEvent) => void
}

function NotificationItem({ notification, onClick, onDismiss }: NotificationItemProps) {
    const { type, entryTitle, date, timestamp, read } = notification
    const icon = getNotificationIcon(type)
    const label = getNotificationLabel(type)
    const timeAgo = formatNotificationTime(timestamp)

    return (
        // biome-ignore lint/a11y/useSemanticElements: Using div with role="button" because we need a nested dismiss button, and HTML doesn't allow nested <button> elements
        <div
            className={`notification-item notification-item-${type}${read ? " read" : ""}`}
            onClick={onClick}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                    onClick()
                }
            }}
        >
            <span className={`notification-item-icon notification-item-icon-${type}`}>{icon}</span>
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
                onClick={onDismiss}
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
