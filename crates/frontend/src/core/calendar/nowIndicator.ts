/**
 * Pure functions for the "now" indicator in the schedule view.
 * Calculates position, formatting, and scroll offset for the current time line.
 */

import { isSameDay } from "./dates"
import { MINUTES_IN_DAY } from "./scheduleLayout"

/**
 * Calculate the vertical position of the current time as a percentage (0–100).
 */
export function calculateNowPositionPercent(hours: number, minutes: number): number {
  return ((hours * 60 + minutes) / MINUTES_IN_DAY) * 100
}

/**
 * Find the index of today's column in the rendered dates array.
 * Returns null if today is not among the rendered dates.
 */
export function findTodayColumnIndex(renderedDates: Date[], today: Date): number | null {
  const idx = renderedDates.findIndex((d) => isSameDay(d, today))
  return idx === -1 ? null : idx
}

/**
 * Format the current time as a 12-hour label (e.g. "6:38 PM").
 */
export function formatNowLabel(hours: number, minutes: number): string {
  const period = hours < 12 ? "AM" : "PM"
  let h = hours % 12
  if (h === 0) h = 12
  const m = String(minutes).padStart(2, "0")
  return `${h}:${m} ${period}`
}

/**
 * Calculate the scroll offset that places the current time in the upper third of the viewport.
 * The result is clamped to [0, totalHeight - viewportHeight].
 */
export function calculateScrollToCurrentTime(
  hours: number,
  minutes: number,
  viewportHeight: number,
  totalHeight: number,
): number {
  const timeOffset = ((hours * 60 + minutes) / MINUTES_IN_DAY) * totalHeight
  const target = timeOffset - viewportHeight / 3
  return Math.max(0, Math.min(target, totalHeight - viewportHeight))
}
