/**
 * Pure functions for day container display logic.
 * These functions compute day header information without side effects.
 */

import { getDayOfMonth, getDayOfWeek, isSameDay } from "./dates"
import { DAY_NAMES } from "./types"

/**
 * Display information for a day header.
 */
export interface DayDisplayInfo {
  dayNumber: number
  dayName: string
  isToday: boolean
  isHighlighted?: boolean
}

/**
 * Check if a date is today.
 * This is a wrapper around the existing isSameDay function from dates.ts.
 */
export function isDayToday(date: Date): boolean {
  return isSameDay(date, new Date())
}

/**
 * Get all display information for a day header.
 * Returns the day number, short day name, and today status.
 */
export function getDayDisplayInfo(date: Date): DayDisplayInfo {
  const dayNumber = getDayOfMonth(date)
  const dayOfWeek = getDayOfWeek(date)
  const dayName = DAY_NAMES[dayOfWeek].toUpperCase()
  const isToday = isDayToday(date)

  return {
    dayNumber,
    dayName,
    isToday,
  }
}
