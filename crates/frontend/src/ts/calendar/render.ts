/**
 * DOM rendering functions for the calendar.
 * This is the Imperative Shell - handles all DOM manipulation.
 */

import {
    formatDateKey,
    getDayOfMonth,
    getDayOfWeek,
    getMonth,
    getYear,
    isToday,
} from "../core/calendar/dates"
import { sortDayEntries } from "../core/calendar/entries"
import type { ServerEntry } from "../core/calendar/types"
import { DAY_NAMES, MONTH_NAMES } from "../core/calendar/types"

/**
 * DOM element references for the calendar.
 */
export interface CalendarElements {
    container: HTMLElement
    grid: HTMLElement
    header: HTMLElement
    monthLabel: HTMLElement
    prevButton: HTMLElement
    nextButton: HTMLElement
    todayButton: HTMLElement
    fab: HTMLElement
}

/**
 * Get calendar DOM elements by their IDs.
 */
export function getCalendarElements(): CalendarElements | null {
    const container = document.getElementById("calendar-container")
    const grid = document.getElementById("calendar-grid")
    const header = document.getElementById("calendar-header")
    const monthLabel = document.getElementById("month-label")
    const prevButton = document.getElementById("prev-btn")
    const nextButton = document.getElementById("next-btn")
    const todayButton = document.getElementById("today-btn")
    const fab = document.getElementById("add-entry-fab")

    if (
        !container ||
        !grid ||
        !header ||
        !monthLabel ||
        !prevButton ||
        !nextButton ||
        !todayButton ||
        !fab
    ) {
        return null
    }

    return {
        container,
        grid,
        header,
        monthLabel,
        prevButton,
        nextButton,
        todayButton,
        fab,
    }
}

/**
 * Update the month label in the header.
 */
export function updateMonthLabel(element: HTMLElement, date: Date): void {
    const month = MONTH_NAMES[getMonth(date)]
    const year = getYear(date)
    element.textContent = `${month} ${year}`
}

/**
 * Create a day column element.
 */
export function createDayColumn(date: Date): HTMLElement {
    const column = document.createElement("div")
    column.className = "day-column"
    column.dataset.date = formatDateKey(date)

    if (isToday(date)) {
        column.classList.add("today")
    }

    // Day header
    const dayHeader = document.createElement("div")
    dayHeader.className = "day-header"

    const dayName = document.createElement("span")
    dayName.className = "day-name"
    dayName.textContent = DAY_NAMES[getDayOfWeek(date)]

    const dayNumber = document.createElement("span")
    dayNumber.className = "day-number"
    dayNumber.textContent = String(getDayOfMonth(date))

    dayHeader.appendChild(dayName)
    dayHeader.appendChild(dayNumber)

    // Entries container
    const entriesContainer = document.createElement("div")
    entriesContainer.className = "day-entries"

    column.appendChild(dayHeader)
    column.appendChild(entriesContainer)

    return column
}

/**
 * Create an entry tile element.
 */
export function createEntryTile(entry: ServerEntry): HTMLElement {
    const tile = document.createElement("div")
    tile.className = "entry-tile"
    tile.dataset.entryId = entry.id
    tile.dataset.calendarId = entry.calendarId

    // Add type-specific classes
    if (entry.isAllDay) {
        tile.classList.add("all-day")
    }
    if (entry.isMultiDay) {
        tile.classList.add("multi-day")
    }
    if (entry.isTask) {
        tile.classList.add("task")
    }
    if (entry.completed) {
        tile.classList.add("completed")
    }

    // Apply color if present
    if (entry.color) {
        tile.style.setProperty("--entry-color", entry.color)
    }

    // Time display
    if (entry.startTime && !entry.isAllDay) {
        const timeSpan = document.createElement("span")
        timeSpan.className = "entry-time"
        timeSpan.textContent = entry.startTime
        tile.appendChild(timeSpan)
    }

    // Title
    const titleSpan = document.createElement("span")
    titleSpan.className = "entry-title"
    titleSpan.textContent = entry.title
    tile.appendChild(titleSpan)

    // Task checkbox
    if (entry.isTask) {
        const checkbox = document.createElement("input")
        checkbox.type = "checkbox"
        checkbox.className = "task-checkbox"
        checkbox.checked = entry.completed
        checkbox.dataset.entryId = entry.id
        tile.prepend(checkbox)
    }

    return tile
}

/**
 * Render entries into a day column.
 */
export function renderEntriesInColumn(column: HTMLElement, entries: ServerEntry[]): void {
    const container = column.querySelector(".day-entries")
    if (!container) return

    // Clear existing entries
    container.innerHTML = ""

    // Sort and render entries
    const sorted = sortDayEntries(entries)
    for (const entry of sorted) {
        const tile = createEntryTile(entry)
        container.appendChild(tile)
    }
}

/**
 * Render the calendar grid with day columns.
 */
export function renderGrid(
    grid: HTMLElement,
    dates: Date[],
    entryCache: Map<string, ServerEntry[]>,
): void {
    // Clear existing content
    grid.innerHTML = ""

    // Create columns for each date
    for (const date of dates) {
        const column = createDayColumn(date)
        const dateKey = formatDateKey(date)
        const entries = entryCache.get(dateKey) || []
        renderEntriesInColumn(column, entries)
        grid.appendChild(column)
    }
}

/**
 * Apply a CSS transform to the grid for animation.
 */
export function setGridTransform(grid: HTMLElement, transform: string): void {
    grid.style.transform = transform
}

/**
 * Set grid transition for smooth animation.
 */
export function setGridTransition(
    grid: HTMLElement,
    duration: number,
    easing: string = "ease-out",
): void {
    grid.style.transition = `transform ${duration}ms ${easing}`
}

/**
 * Clear grid transition.
 */
export function clearGridTransition(grid: HTMLElement): void {
    grid.style.transition = ""
}

/**
 * Show loading indicator.
 */
export function showLoading(container: HTMLElement): void {
    container.classList.add("loading")
}

/**
 * Hide loading indicator.
 */
export function hideLoading(container: HTMLElement): void {
    container.classList.remove("loading")
}

/**
 * Show error message.
 */
export function showError(container: HTMLElement, message: string): void {
    const errorEl = document.createElement("div")
    errorEl.className = "calendar-error"
    errorEl.textContent = message
    container.prepend(errorEl)

    // Auto-dismiss after 5 seconds
    setTimeout(() => {
        errorEl.remove()
    }, 5000)
}
