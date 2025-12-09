/**
 * CalendarContext - provides shared state to calendar sub-components.
 * This eliminates props drilling through DayColumn to EntryTile.
 */

import type { ServerEntry } from "@core/calendar/types"
import { createContext, useContext, useMemo } from "react"
import type { FlashState } from "../types"

/** Context value shared with calendar sub-components */
export interface CalendarContextValue {
    /** Map of entry IDs to their flash animation state */
    flashStates: Map<string, FlashState>
    /** Callback when an entry is clicked (opens edit modal) */
    onEntryClick: (entry: ServerEntry) => void
    /** Callback when a task entry checkbox is toggled */
    onEntryToggle: (entry: ServerEntry) => void
    /** Whether the viewport is mobile-sized */
    isMobile: boolean
}

/** CalendarContext - null when not inside provider */
const CalendarContext = createContext<CalendarContextValue | null>(null)

/** Props for CalendarProvider */
export interface CalendarProviderProps {
    children: React.ReactNode
    flashStates: Map<string, FlashState>
    onEntryClick: (entry: ServerEntry) => void
    onEntryToggle: (entry: ServerEntry) => void
    isMobile: boolean
}

/**
 * CalendarProvider - wraps calendar sub-components with shared context.
 */
export function CalendarProvider({
    children,
    flashStates,
    onEntryClick,
    onEntryToggle,
    isMobile,
}: CalendarProviderProps) {
    const value = useMemo<CalendarContextValue>(
        () => ({
            flashStates,
            onEntryClick,
            onEntryToggle,
            isMobile,
        }),
        [flashStates, onEntryClick, onEntryToggle, isMobile],
    )

    return <CalendarContext.Provider value={value}>{children}</CalendarContext.Provider>
}

/**
 * Hook to access CalendarContext.
 * Throws if used outside CalendarProvider.
 */
export function useCalendarContext(): CalendarContextValue {
    const ctx = useContext(CalendarContext)
    if (!ctx) {
        throw new Error("useCalendarContext must be used within CalendarProvider")
    }
    return ctx
}
