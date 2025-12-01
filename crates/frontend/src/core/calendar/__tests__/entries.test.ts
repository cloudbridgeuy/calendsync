import { describe, expect, test } from "bun:test"
import {
    filterByCalendar,
    filterByCompletion,
    getEntriesForDate,
    getMissingDateKeys,
    getRequiredDateRange,
    getUniqueCalendarIds,
    groupEntriesByDate,
    isCompletedEntry,
    isTaskEntry,
    mergeEntryCache,
    serverDaysToMap,
    sortDayEntries,
} from "../entries"
import type { ServerDay, ServerEntry } from "../types"

// Helper to create test entries
function createEntry(overrides: Partial<ServerEntry> = {}): ServerEntry {
    return {
        id: "test-id",
        calendarId: "cal-1",
        kind: "event",
        completed: false,
        isMultiDay: false,
        isAllDay: false,
        isTimed: true,
        isTask: false,
        title: "Test Entry",
        description: null,
        location: null,
        color: null,
        date: "2024-01-15",
        startTime: "10:00",
        endTime: "11:00",
        multiDayStart: null,
        multiDayEnd: null,
        multiDayStartDate: null,
        multiDayEndDate: null,
        ...overrides,
    }
}

describe("groupEntriesByDate", () => {
    test("groups entries by date", () => {
        const entries = [
            createEntry({ id: "1", date: "2024-01-15" }),
            createEntry({ id: "2", date: "2024-01-16" }),
            createEntry({ id: "3", date: "2024-01-15" }),
        ]

        const grouped = groupEntriesByDate(entries)

        expect(grouped.size).toBe(2)
        expect(grouped.get("2024-01-15")?.length).toBe(2)
        expect(grouped.get("2024-01-16")?.length).toBe(1)
    })

    test("returns empty map for empty array", () => {
        const grouped = groupEntriesByDate([])
        expect(grouped.size).toBe(0)
    })
})

describe("sortDayEntries", () => {
    test("sorts all-day entries first", () => {
        const entries = [
            createEntry({ id: "1", isAllDay: false, startTime: "10:00" }),
            createEntry({ id: "2", isAllDay: true, startTime: null }),
        ]

        const sorted = sortDayEntries(entries)

        expect(sorted[0].id).toBe("2")
        expect(sorted[1].id).toBe("1")
    })

    test("sorts by start time", () => {
        const entries = [
            createEntry({ id: "1", startTime: "14:00" }),
            createEntry({ id: "2", startTime: "09:00" }),
            createEntry({ id: "3", startTime: "11:00" }),
        ]

        const sorted = sortDayEntries(entries)

        expect(sorted[0].id).toBe("2")
        expect(sorted[1].id).toBe("3")
        expect(sorted[2].id).toBe("1")
    })

    test("sorts by title when times are equal", () => {
        const entries = [
            createEntry({ id: "1", startTime: "10:00", title: "Zebra" }),
            createEntry({ id: "2", startTime: "10:00", title: "Apple" }),
        ]

        const sorted = sortDayEntries(entries)

        expect(sorted[0].id).toBe("2")
        expect(sorted[1].id).toBe("1")
    })

    test("does not mutate original array", () => {
        const entries = [
            createEntry({ id: "1", startTime: "14:00" }),
            createEntry({ id: "2", startTime: "09:00" }),
        ]
        const originalFirst = entries[0].id

        sortDayEntries(entries)

        expect(entries[0].id).toBe(originalFirst)
    })
})

describe("getEntriesForDate", () => {
    test("returns entries matching date", () => {
        const entries = [
            createEntry({ id: "1", date: "2024-01-15" }),
            createEntry({ id: "2", date: "2024-01-16" }),
        ]

        const result = getEntriesForDate(entries, "2024-01-15")

        expect(result.length).toBe(1)
        expect(result[0].id).toBe("1")
    })

    test("includes multi-day entries spanning the date", () => {
        const entries = [
            createEntry({
                id: "1",
                date: "2024-01-14",
                isMultiDay: true,
                multiDayStartDate: "2024-01-14",
                multiDayEndDate: "2024-01-16",
            }),
        ]

        const result = getEntriesForDate(entries, "2024-01-15")

        expect(result.length).toBe(1)
        expect(result[0].id).toBe("1")
    })

    test("excludes multi-day entries not spanning the date", () => {
        const entries = [
            createEntry({
                id: "1",
                date: "2024-01-10",
                isMultiDay: true,
                multiDayStartDate: "2024-01-10",
                multiDayEndDate: "2024-01-12",
            }),
        ]

        const result = getEntriesForDate(entries, "2024-01-15")

        expect(result.length).toBe(0)
    })
})

describe("serverDaysToMap", () => {
    test("converts array to map", () => {
        const days: ServerDay[] = [
            { date: "2024-01-15", entries: [createEntry({ id: "1" })] },
            { date: "2024-01-16", entries: [createEntry({ id: "2" })] },
        ]

        const map = serverDaysToMap(days)

        expect(map.size).toBe(2)
        expect(map.get("2024-01-15")?.[0].id).toBe("1")
        expect(map.get("2024-01-16")?.[0].id).toBe("2")
    })
})

describe("mergeEntryCache", () => {
    test("adds new entries to cache", () => {
        const existing = new Map<string, ServerEntry[]>()
        existing.set("2024-01-15", [createEntry({ id: "1" })])

        const newDays: ServerDay[] = [{ date: "2024-01-16", entries: [createEntry({ id: "2" })] }]

        const merged = mergeEntryCache(existing, newDays)

        expect(merged.size).toBe(2)
        expect(merged.get("2024-01-15")?.[0].id).toBe("1")
        expect(merged.get("2024-01-16")?.[0].id).toBe("2")
    })

    test("replaces existing entries for same date", () => {
        const existing = new Map<string, ServerEntry[]>()
        existing.set("2024-01-15", [createEntry({ id: "old" })])

        const newDays: ServerDay[] = [{ date: "2024-01-15", entries: [createEntry({ id: "new" })] }]

        const merged = mergeEntryCache(existing, newDays)

        expect(merged.get("2024-01-15")?.[0].id).toBe("new")
    })

    test("does not mutate original cache", () => {
        const existing = new Map<string, ServerEntry[]>()
        existing.set("2024-01-15", [createEntry({ id: "1" })])

        const newDays: ServerDay[] = [{ date: "2024-01-16", entries: [createEntry({ id: "2" })] }]

        mergeEntryCache(existing, newDays)

        expect(existing.size).toBe(1)
    })
})

describe("getMissingDateKeys", () => {
    test("returns keys not in cache", () => {
        const cache = new Map<string, ServerEntry[]>()
        cache.set("2024-01-15", [])

        const keys = ["2024-01-15", "2024-01-16", "2024-01-17"]
        const missing = getMissingDateKeys(cache, keys)

        expect(missing).toEqual(["2024-01-16", "2024-01-17"])
    })

    test("returns empty array when all keys present", () => {
        const cache = new Map<string, ServerEntry[]>()
        cache.set("2024-01-15", [])
        cache.set("2024-01-16", [])

        const missing = getMissingDateKeys(cache, ["2024-01-15", "2024-01-16"])

        expect(missing).toEqual([])
    })
})

describe("getRequiredDateRange", () => {
    test("returns range centered on date", () => {
        const center = new Date(2024, 0, 15) // Jan 15
        const result = getRequiredDateRange(center, 7, 3)

        // 7 visible days -> half is 3
        // With 3 buffer: need 6 days before and after
        expect(result.start).toBe("2024-01-09")
        expect(result.end).toBe("2024-01-21")
    })
})

describe("isTaskEntry", () => {
    test("returns true for task", () => {
        const entry = createEntry({ isTask: true })
        expect(isTaskEntry(entry)).toBe(true)
    })

    test("returns false for event", () => {
        const entry = createEntry({ isTask: false })
        expect(isTaskEntry(entry)).toBe(false)
    })
})

describe("isCompletedEntry", () => {
    test("returns true for completed entry", () => {
        const entry = createEntry({ completed: true })
        expect(isCompletedEntry(entry)).toBe(true)
    })

    test("returns false for incomplete entry", () => {
        const entry = createEntry({ completed: false })
        expect(isCompletedEntry(entry)).toBe(false)
    })
})

describe("filterByCompletion", () => {
    test("filters completed entries", () => {
        const entries = [
            createEntry({ id: "1", completed: true }),
            createEntry({ id: "2", completed: false }),
        ]

        const completed = filterByCompletion(entries, true)

        expect(completed.length).toBe(1)
        expect(completed[0].id).toBe("1")
    })

    test("filters incomplete entries", () => {
        const entries = [
            createEntry({ id: "1", completed: true }),
            createEntry({ id: "2", completed: false }),
        ]

        const incomplete = filterByCompletion(entries, false)

        expect(incomplete.length).toBe(1)
        expect(incomplete[0].id).toBe("2")
    })
})

describe("filterByCalendar", () => {
    test("filters by calendar ID", () => {
        const entries = [
            createEntry({ id: "1", calendarId: "cal-1" }),
            createEntry({ id: "2", calendarId: "cal-2" }),
            createEntry({ id: "3", calendarId: "cal-1" }),
        ]

        const filtered = filterByCalendar(entries, "cal-1")

        expect(filtered.length).toBe(2)
        expect(filtered[0].id).toBe("1")
        expect(filtered[1].id).toBe("3")
    })
})

describe("getUniqueCalendarIds", () => {
    test("returns unique calendar IDs", () => {
        const entries = [
            createEntry({ calendarId: "cal-1" }),
            createEntry({ calendarId: "cal-2" }),
            createEntry({ calendarId: "cal-1" }),
        ]

        const ids = getUniqueCalendarIds(entries)

        expect(ids.length).toBe(2)
        expect(ids).toContain("cal-1")
        expect(ids).toContain("cal-2")
    })

    test("returns empty array for no entries", () => {
        const ids = getUniqueCalendarIds([])
        expect(ids).toEqual([])
    })
})
