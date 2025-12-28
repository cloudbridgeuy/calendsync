/**
 * Pure functions for computing the all-day section layout.
 * Handles categorization, overflow calculation, and toggle text formatting.
 */

import type { ServerEntry } from "./types"

/** Maximum visible entries when collapsed */
export const MAX_VISIBLE_ALL_DAY = 3

/** Entry categories for the all-day section */
export interface AllDayCategorized {
  /** Multi-day and all-day entries (sorted: multi-day first) */
  events: ServerEntry[]
  /** Task entries */
  tasks: ServerEntry[]
}

/** Summary of what to display in the all-day section */
export interface AllDaySummary {
  /** Entries to show (up to MAX_VISIBLE_ALL_DAY when collapsed) */
  visibleEvents: ServerEntry[]
  /** Number of hidden events beyond the visible ones */
  hiddenEventCount: number
  /** Task entries (shown only when expanded) */
  tasks: ServerEntry[]
}

/**
 * Categorize all-day entries into events and tasks.
 * Events are sorted with multi-day first, then all-day.
 */
export function categorizeAllDayEntries(entries: ServerEntry[]): AllDayCategorized {
  const events: ServerEntry[] = []
  const tasks: ServerEntry[] = []

  for (const entry of entries) {
    if (entry.isTask) {
      tasks.push(entry)
    } else if (entry.isAllDay || entry.isMultiDay) {
      events.push(entry)
    }
  }

  // Sort events: multi-day first, then all-day
  events.sort((a, b) => {
    if (a.isMultiDay && !b.isMultiDay) return -1
    if (!a.isMultiDay && b.isMultiDay) return 1
    return 0
  })

  return { events, tasks }
}

/**
 * Compute what to display in the all-day section for a single day.
 */
export function computeAllDaySummary(entries: ServerEntry[], showOverflow: boolean): AllDaySummary {
  const { events, tasks } = categorizeAllDayEntries(entries)

  const visibleEvents = showOverflow ? events : events.slice(0, MAX_VISIBLE_ALL_DAY)
  const hiddenEventCount = showOverflow ? 0 : Math.max(0, events.length - MAX_VISIBLE_ALL_DAY)

  return {
    visibleEvents,
    hiddenEventCount,
    tasks,
  }
}

/**
 * Format the overflow toggle text.
 * Returns null if no overflow.
 */
export function formatOverflowToggle(hiddenCount: number, isExpanded: boolean): string | null {
  if (hiddenCount === 0 && !isExpanded) return null
  return isExpanded ? "Show less" : `(+${hiddenCount} more)`
}

/**
 * Format the tasks toggle text.
 * Returns null if no tasks.
 */
export function formatTasksToggle(taskCount: number, isExpanded: boolean): string | null {
  if (taskCount === 0) return null
  const plural = taskCount === 1 ? "task" : "tasks"
  return isExpanded ? `${taskCount} ${plural}` : `(${taskCount} ${plural})`
}
