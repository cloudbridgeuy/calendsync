/**
 * Pure functions for Schedule view layout calculations.
 * Handles time positioning, overlap detection, and entry separation.
 */

import type { ServerEntry } from "./types"

// ============================================================================
// Constants
// ============================================================================

/** Height of each hour row in pixels */
export const HOUR_HEIGHT_PX = 60

/** Total hours in a day */
export const HOURS_IN_DAY = 24

/** Total minutes in a day */
export const MINUTES_IN_DAY = 1440

/** Default hour to scroll to on load (8 AM) */
export const DEFAULT_SCROLL_HOUR = 8

// ============================================================================
// Types
// ============================================================================

/** Position information for a timed entry */
export interface TimePosition {
  /** Pixels from top of the time grid */
  top: number
  /** Height in pixels */
  height: number
}

/** Overlap column assignment for an entry */
export interface OverlapColumn {
  /** Column index (0-based) */
  columnIndex: number
  /** Total number of columns in this overlap group */
  totalColumns: number
}

/** Entries separated by type for schedule view */
export interface SeparatedEntries {
  /** All-day entries */
  allDay: ServerEntry[]
  /** Multi-day entries */
  multiDay: ServerEntry[]
  /** Task entries */
  tasks: ServerEntry[]
  /** Timed entries (have start and end time) */
  timed: ServerEntry[]
}

// ============================================================================
// Time Parsing Functions
// ============================================================================

/**
 * Parse a time string (HH:MM:SS or HH:MM) to minutes from midnight.
 * Returns 0 if the time string is invalid or null.
 */
export function parseTimeToMinutes(timeStr: string | null): number {
  if (!timeStr) return 0

  const parts = timeStr.split(":")
  if (parts.length < 2) return 0

  const hours = Number.parseInt(parts[0], 10)
  const minutes = Number.parseInt(parts[1], 10)

  if (Number.isNaN(hours) || Number.isNaN(minutes)) return 0

  return hours * 60 + minutes
}

/**
 * Calculate duration in minutes between start and end times.
 * Returns 0 if either time is invalid.
 */
export function calculateDuration(startTime: string | null, endTime: string | null): number {
  const startMinutes = parseTimeToMinutes(startTime)
  const endMinutes = parseTimeToMinutes(endTime)

  // Handle overnight events (end < start)
  if (endMinutes < startMinutes) {
    return MINUTES_IN_DAY - startMinutes + endMinutes
  }

  return endMinutes - startMinutes
}

// ============================================================================
// Position Calculation Functions
// ============================================================================

/**
 * Calculate the position and height of a timed entry in the schedule grid.
 */
export function calculateTimePosition(
  startTime: string | null,
  endTime: string | null,
  hourHeight: number = HOUR_HEIGHT_PX,
): TimePosition {
  const startMinutes = parseTimeToMinutes(startTime)
  const durationMinutes = calculateDuration(startTime, endTime)

  // Convert minutes to pixels
  const pixelsPerMinute = hourHeight / 60
  const top = startMinutes * pixelsPerMinute
  const height = Math.max(durationMinutes * pixelsPerMinute, hourHeight / 4) // Minimum height of 15 minutes

  return { top, height }
}

/**
 * Calculate scroll position to center on a specific hour.
 */
export function calculateScrollToHour(hour: number, hourHeight: number = HOUR_HEIGHT_PX): number {
  return hour * hourHeight
}

// ============================================================================
// Entry Separation Functions
// ============================================================================

/**
 * Separate entries by their type for schedule view rendering.
 * - allDay: entries that span the full day
 * - multiDay: entries that span multiple days
 * - tasks: task entries (checkbox items)
 * - timed: entries with specific start/end times
 */
export function separateEntriesByType(entries: ServerEntry[]): SeparatedEntries {
  const allDay: ServerEntry[] = []
  const multiDay: ServerEntry[] = []
  const tasks: ServerEntry[] = []
  const timed: ServerEntry[] = []

  for (const entry of entries) {
    if (entry.isTask) {
      tasks.push(entry)
    } else if (entry.isAllDay) {
      allDay.push(entry)
    } else if (entry.isMultiDay) {
      multiDay.push(entry)
    } else if (entry.isTimed && entry.startTime && entry.endTime) {
      timed.push(entry)
    }
  }

  return { allDay, multiDay, tasks, timed }
}

// ============================================================================
// Overlap Detection Functions
// ============================================================================

/**
 * Check if two time ranges overlap.
 */
function timeRangesOverlap(start1: number, end1: number, start2: number, end2: number): boolean {
  return start1 < end2 && start2 < end1
}

/**
 * Detect overlapping entries and assign column positions.
 * Returns a Map of entry ID to overlap column information.
 *
 * Algorithm:
 * 1. Sort entries by start time
 * 2. For each entry, find all overlapping entries
 * 3. Assign column indices to avoid visual overlap
 */
export function detectOverlappingEntries(entries: ServerEntry[]): Map<string, OverlapColumn> {
  const result = new Map<string, OverlapColumn>()

  if (entries.length === 0) return result

  // Sort by start time, then by duration (longer first)
  const sorted = [...entries].sort((a, b) => {
    const startA = parseTimeToMinutes(a.startTime)
    const startB = parseTimeToMinutes(b.startTime)
    if (startA !== startB) return startA - startB

    const durA = calculateDuration(a.startTime, a.endTime)
    const durB = calculateDuration(b.startTime, b.endTime)
    return durB - durA // Longer duration first
  })

  // Track active intervals and their column assignments
  const columns: { entry: ServerEntry; endMinutes: number; columnIndex: number }[] = []

  for (const entry of sorted) {
    const startMinutes = parseTimeToMinutes(entry.startTime)
    const endMinutes = startMinutes + calculateDuration(entry.startTime, entry.endTime)

    // Remove entries that have ended before this one starts
    const activeColumns = columns.filter((col) => col.endMinutes > startMinutes)

    // Find the first available column
    const usedColumns = new Set(activeColumns.map((col) => col.columnIndex))
    let columnIndex = 0
    while (usedColumns.has(columnIndex)) {
      columnIndex++
    }

    // Add this entry to tracking
    columns.length = 0
    columns.push(...activeColumns, { entry, endMinutes, columnIndex })

    // Calculate total columns needed for all overlapping entries
    const maxColumn = Math.max(...columns.map((col) => col.columnIndex)) + 1

    // Update all active entries with the new total column count
    for (const col of columns) {
      result.set(col.entry.id, {
        columnIndex: col.columnIndex,
        totalColumns: maxColumn,
      })
    }
  }

  return result
}

/**
 * Get entries that overlap with a specific entry.
 */
export function getOverlappingEntries(
  entry: ServerEntry,
  allEntries: ServerEntry[],
): ServerEntry[] {
  const entryStart = parseTimeToMinutes(entry.startTime)
  const entryEnd = entryStart + calculateDuration(entry.startTime, entry.endTime)

  return allEntries.filter((other) => {
    if (other.id === entry.id) return false

    const otherStart = parseTimeToMinutes(other.startTime)
    const otherEnd = otherStart + calculateDuration(other.startTime, other.endTime)

    return timeRangesOverlap(entryStart, entryEnd, otherStart, otherEnd)
  })
}

// ============================================================================
// Layout Calculation Functions
// ============================================================================

/**
 * Calculate the total height of the schedule grid.
 */
export function calculateGridHeight(hourHeight: number = HOUR_HEIGHT_PX): number {
  return HOURS_IN_DAY * hourHeight
}

/**
 * Calculate width and left offset for an entry based on overlap columns.
 */
export function calculateEntryWidth(
  overlapColumn: OverlapColumn,
  containerWidth: number,
): { width: number; left: number } {
  const { columnIndex, totalColumns } = overlapColumn
  const width = containerWidth / totalColumns
  const left = columnIndex * width

  return { width, left }
}

/**
 * Format hour for display (e.g., "9 AM", "12 PM", "6 PM").
 */
export function formatHourLabel(hour: number): string {
  if (hour === 0) return "12 AM"
  if (hour === 12) return "12 PM"
  if (hour < 12) return `${hour} AM`
  return `${hour - 12} PM`
}

/**
 * Generate hour labels for the schedule view (0-23).
 */
export function generateHourLabels(): string[] {
  return Array.from({ length: HOURS_IN_DAY }, (_, i) => formatHourLabel(i))
}
