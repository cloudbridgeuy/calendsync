/**
 * Accessibility utilities for calendar navigation.
 * Pure functions for ARIA announcements and focus management.
 */

import { getDayDisplayInfo } from "./dayContainer"

/**
 * Generate ARIA announcement text for navigation to a date.
 */
export function generateNavigationAnnouncement(date: Date): string {
  const info = getDayDisplayInfo(date)

  if (info.isToday) {
    return `Navigated to today, ${info.dayName} ${info.dayNumber}`
  }

  return `Navigated to ${info.dayName} ${info.dayNumber}`
}

/**
 * Build selector for first focusable entry in a day column.
 */
export function buildFirstEntrySelector(dateKey: string): string {
  return `.day-column[data-date="${dateKey}"] .entry-tile[tabindex="0"]`
}

/**
 * Build selector for a day header by date.
 */
export function buildDayHeaderSelector(dateKey: string): string {
  return `.day-container-header[data-date="${dateKey}"]`
}

/**
 * Build selector for a day column by date.
 */
export function buildDayColumnSelector(dateKey: string): string {
  return `.day-column[data-date="${dateKey}"]`
}

/**
 * Focus the first entry in a day, or the header if no entries.
 * Returns true if focus was set, false otherwise.
 */
export function focusDayElement(dateKey: string): boolean {
  // Try first entry
  const entry = document.querySelector<HTMLElement>(buildFirstEntrySelector(dateKey))
  if (entry) {
    entry.focus({ preventScroll: true })
    return true
  }

  // Fall back to header
  const header = document.querySelector<HTMLElement>(buildDayHeaderSelector(dateKey))
  if (header) {
    header.focus({ preventScroll: true })
    return true
  }

  return false
}
