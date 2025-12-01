/**
 * Calendar entry from the server.
 * This matches the data structure rendered by Askama templates.
 */
export interface ServerEntry {
    id: string
    calendarId: string
    kind: string
    completed: boolean
    isMultiDay: boolean
    isAllDay: boolean
    isTimed: boolean
    isTask: boolean
    title: string
    description: string | null
    location: string | null
    color: string | null
    date: string
    startTime: string | null
    endTime: string | null
    multiDayStart: string | null
    multiDayEnd: string | null
    multiDayStartDate: string | null
    multiDayEndDate: string | null
}

/**
 * A day with its entries from the server.
 */
export interface ServerDay {
    date: string
    entries: ServerEntry[]
}

/**
 * Calendar layout constants.
 */
export interface LayoutConstants {
    minDayWidth: number
    mobileBreakpoint: number
    swipeThreshold: number
    velocityThreshold: number
    animationDuration: number
    mobileBuffer: number
}

/**
 * Default layout constants.
 */
export const DEFAULT_LAYOUT_CONSTANTS: LayoutConstants = {
    minDayWidth: 250,
    mobileBreakpoint: 768,
    swipeThreshold: 50,
    velocityThreshold: 0.3,
    animationDuration: 200,
    mobileBuffer: 30,
}

/**
 * Day name arrays for display.
 */
export const DAY_NAMES = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] as const

export const DAY_NAMES_FULL = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
] as const

export const MONTH_NAMES = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
] as const
