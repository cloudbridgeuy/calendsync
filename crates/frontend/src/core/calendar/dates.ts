/**
 * Pure date manipulation functions.
 * These functions have no side effects and are fully testable.
 */

/**
 * Add a number of days to a date.
 * Returns a new Date object.
 */
export function addDays(date: Date, days: number): Date {
  const result = new Date(date)
  result.setDate(result.getDate() + days)
  return result
}

/**
 * Check if two dates are the same day.
 */
export function isSameDay(d1: Date, d2: Date): boolean {
  return (
    d1.getFullYear() === d2.getFullYear() &&
    d1.getMonth() === d2.getMonth() &&
    d1.getDate() === d2.getDate()
  )
}

/**
 * Format a date as YYYY-MM-DD string.
 * This is used as a key for the entry cache.
 */
export function formatDateKey(date: Date): string {
  const y = date.getFullYear()
  const m = String(date.getMonth() + 1).padStart(2, "0")
  const d = String(date.getDate()).padStart(2, "0")
  return `${y}-${m}-${d}`
}

/**
 * Parse a YYYY-MM-DD string to a Date object.
 * Sets time to midnight local time.
 */
export function parseDateKey(dateKey: string): Date {
  const [year, month, day] = dateKey.split("-").map(Number)
  const date = new Date(year, month - 1, day)
  date.setHours(0, 0, 0, 0)
  return date
}

/**
 * Get the start of day (midnight) for a date.
 * Returns a new Date object.
 */
export function startOfDay(date: Date): Date {
  const result = new Date(date)
  result.setHours(0, 0, 0, 0)
  return result
}

/**
 * Get the day of week index (0 = Sunday, 6 = Saturday).
 */
export function getDayOfWeek(date: Date): number {
  return date.getDay()
}

/**
 * Get the day of month (1-31).
 */
export function getDayOfMonth(date: Date): number {
  return date.getDate()
}

/**
 * Get the month index (0-11).
 */
export function getMonth(date: Date): number {
  return date.getMonth()
}

/**
 * Get the full year.
 */
export function getYear(date: Date): number {
  return date.getFullYear()
}

/**
 * Check if a date is today.
 */
export function isToday(date: Date): boolean {
  return isSameDay(date, new Date())
}

/**
 * Get an array of dates for a range.
 * Includes both start and end dates.
 */
export function getDateRange(start: Date, end: Date): Date[] {
  const dates: Date[] = []
  let current = startOfDay(start)
  const endDay = startOfDay(end)

  while (current <= endDay) {
    dates.push(new Date(current))
    current = addDays(current, 1)
  }

  return dates
}

/**
 * Get an array of dates centered around a date.
 * Returns (before + 1 + after) dates.
 */
export function getDatesAround(center: Date, before: number, after: number): Date[] {
  const dates: Date[] = []
  const start = addDays(center, -before)

  for (let i = 0; i <= before + after; i++) {
    dates.push(addDays(start, i))
  }

  return dates
}
