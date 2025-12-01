/**
 * Calendar module entry point.
 * Sets up the window.calendar global object.
 */

import { formatDateKey } from "../core/calendar/dates"
import type { ApiConfig } from "./api"
import { createEntryModalController, getModalElements } from "./entryModal"
import { createCalendarController } from "./events"
import { getCalendarElements } from "./render"

/**
 * Calendar public API exposed on window.calendar
 */
export interface CalendarApi {
    /** Get the current highlighted/center date */
    getCenterDate: () => Date
    /** Get the number of visible days */
    getVisibleDays: () => number
    /** Navigate by a number of days (positive = forward, negative = back) */
    navigateDays: (offset: number) => void
    /** Jump to today */
    goToToday: () => void
    /** Jump to a specific date */
    goToDate: (date: Date) => void
    /** Refresh calendar data */
    refresh: () => void
    /** Open the create entry modal */
    openCreateModal: (defaultDate?: string) => void
    /** Check if modal is open */
    isModalOpen: () => boolean
}

/**
 * Initialize the calendar.
 * Called from the HTML page after DOM is ready.
 */
export function initCalendar(calendarId: string): CalendarApi | null {
    // Get DOM elements
    const elements = getCalendarElements()
    if (!elements) {
        console.error("Calendar: Required DOM elements not found")
        return null
    }

    const modalElements = getModalElements()
    if (!modalElements) {
        console.error("Calendar: Modal DOM elements not found")
        return null
    }

    // API configuration
    const config: ApiConfig = {
        baseUrl: window.location.origin,
        calendarId,
    }

    // Create calendar controller
    const calendar = createCalendarController(elements, config)

    // Create modal controller
    const modal = createEntryModalController(modalElements, config, () => {
        calendar.refresh()
    })

    // Set up FAB click handler
    elements.fab.addEventListener("click", () => {
        const centerDate = calendar.getCenterDate()
        modal.openCreate(formatDateKey(centerDate))
    })

    // Public API
    const api: CalendarApi = {
        getCenterDate: calendar.getCenterDate,
        getVisibleDays: calendar.getVisibleDays,
        navigateDays: calendar.navigateDays,
        goToToday: calendar.goToToday,
        goToDate: calendar.goToDate,
        refresh: calendar.refresh,
        openCreateModal: modal.openCreate,
        isModalOpen: modal.isOpen,
    }

    return api
}

// Expose to window for use from HTML
declare global {
    interface Window {
        initCalendar: typeof initCalendar
        calendar?: CalendarApi
    }
}

window.initCalendar = initCalendar
