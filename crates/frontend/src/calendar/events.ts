/**
 * Event handling and state management for the calendar.
 * This is the Imperative Shell - handles user interaction.
 */

import { addDays, formatDateKey, getDatesAround } from "../core/calendar/dates"
import { getMissingDateKeys, getRequiredDateRange, mergeEntryCache } from "../core/calendar/entries"
import { calculateVisibleDays, shouldNavigateFromSwipe } from "../core/calendar/layout"
import type { ServerEntry } from "../core/calendar/types"
import { DEFAULT_LAYOUT_CONSTANTS } from "../core/calendar/types"
import type { ApiConfig } from "./api"
import { fetchEntries, toggleTaskCompletion } from "./api"
import type { CalendarElements } from "./render"
import {
    clearGridTransition,
    hideLoading,
    renderGrid,
    setGridTransform,
    showError,
    showLoading,
    updateMonthLabel,
} from "./render"

/**
 * Calendar state.
 */
export interface CalendarState {
    centerDate: Date
    visibleDays: number
    entryCache: Map<string, ServerEntry[]>
    isLoading: boolean
    containerWidth: number
}

/**
 * Touch tracking for swipe gestures.
 */
interface TouchState {
    startX: number
    startY: number
    startTime: number
    currentX: number
    isTracking: boolean
}

/**
 * Create initial calendar state.
 */
export function createInitialState(containerWidth: number): CalendarState {
    const visibleDays = calculateVisibleDays(containerWidth)
    return {
        centerDate: new Date(),
        visibleDays,
        entryCache: new Map(),
        isLoading: false,
        containerWidth,
    }
}

/**
 * Create the calendar controller.
 */
export function createCalendarController(elements: CalendarElements, config: ApiConfig) {
    let state = createInitialState(elements.container.clientWidth)
    let touchState: TouchState = {
        startX: 0,
        startY: 0,
        startTime: 0,
        currentX: 0,
        isTracking: false,
    }

    // Public API
    const controller = {
        getCenterDate: () => new Date(state.centerDate),
        getVisibleDays: () => state.visibleDays,
        navigateDays,
        goToToday,
        goToDate,
        refresh,
        destroy,
    }

    // Initialize
    init()

    return controller

    // --- Implementation ---

    async function init() {
        // Set up event listeners
        setupNavigationListeners()
        setupTouchListeners()
        setupResizeListener()
        setupEntryClickListener()

        // Initial render
        await loadAndRender()
    }

    function setupNavigationListeners() {
        elements.prevButton.addEventListener("click", () => navigateDays(-1))
        elements.nextButton.addEventListener("click", () => navigateDays(1))
        elements.todayButton.addEventListener("click", goToToday)

        // Keyboard navigation
        document.addEventListener("keydown", handleKeydown)
    }

    function handleKeydown(e: KeyboardEvent) {
        // Ignore if user is typing in an input
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
            return
        }

        switch (e.key) {
            case "ArrowLeft":
                navigateDays(-1)
                break
            case "ArrowRight":
                navigateDays(1)
                break
            case "t":
            case "T":
                goToToday()
                break
        }
    }

    function setupTouchListeners() {
        const grid = elements.grid

        grid.addEventListener("touchstart", handleTouchStart, { passive: true })
        grid.addEventListener("touchmove", handleTouchMove, { passive: false })
        grid.addEventListener("touchend", handleTouchEnd, { passive: true })
        grid.addEventListener("touchcancel", handleTouchCancel, { passive: true })
    }

    function handleTouchStart(e: TouchEvent) {
        const touch = e.touches[0]
        touchState = {
            startX: touch.clientX,
            startY: touch.clientY,
            startTime: Date.now(),
            currentX: touch.clientX,
            isTracking: true,
        }
    }

    function handleTouchMove(e: TouchEvent) {
        if (!touchState.isTracking) return

        const touch = e.touches[0]
        const deltaX = touch.clientX - touchState.startX
        const deltaY = touch.clientY - touchState.startY

        // If vertical scroll is more prominent, stop tracking horizontal swipe
        if (Math.abs(deltaY) > Math.abs(deltaX) * 1.5) {
            touchState.isTracking = false
            clearGridTransition(elements.grid)
            setGridTransform(elements.grid, "")
            return
        }

        // Prevent default to stop page scroll during horizontal swipe
        if (Math.abs(deltaX) > 10) {
            e.preventDefault()
        }

        touchState.currentX = touch.clientX

        // Apply transform for visual feedback
        const dayWidth = state.containerWidth / state.visibleDays
        const percentOffset = (deltaX / dayWidth) * 100
        setGridTransform(elements.grid, `translateX(${percentOffset}%)`)
    }

    function handleTouchEnd() {
        if (!touchState.isTracking) return

        const deltaX = touchState.currentX - touchState.startX
        const deltaTime = Date.now() - touchState.startTime
        const velocity = deltaX / deltaTime

        const { shouldNavigate, direction } = shouldNavigateFromSwipe(
            deltaX,
            velocity,
            DEFAULT_LAYOUT_CONSTANTS,
        )

        clearGridTransition(elements.grid)
        setGridTransform(elements.grid, "")

        if (shouldNavigate) {
            navigateDays(direction)
        }

        touchState.isTracking = false
    }

    function handleTouchCancel() {
        touchState.isTracking = false
        clearGridTransition(elements.grid)
        setGridTransform(elements.grid, "")
    }

    function setupResizeListener() {
        const resizeObserver = new ResizeObserver((entries) => {
            for (const entry of entries) {
                const newWidth = entry.contentRect.width
                if (newWidth !== state.containerWidth) {
                    handleResize(newWidth)
                }
            }
        })

        resizeObserver.observe(elements.container)
    }

    function handleResize(newWidth: number) {
        const newVisibleDays = calculateVisibleDays(newWidth)

        if (newVisibleDays !== state.visibleDays) {
            state = {
                ...state,
                containerWidth: newWidth,
                visibleDays: newVisibleDays,
            }
            renderCurrentView()
        } else {
            state = { ...state, containerWidth: newWidth }
        }
    }

    function setupEntryClickListener() {
        elements.grid.addEventListener("click", handleEntryClick)
        elements.grid.addEventListener("change", handleTaskCheckbox)
    }

    function handleEntryClick(e: Event) {
        const target = e.target as HTMLElement
        const tile = target.closest(".entry-tile")

        if (!tile || target.classList.contains("task-checkbox")) return

        const entryId = (tile as HTMLElement).dataset.entryId
        if (!entryId) return

        // Find entry data
        const entry = findEntryById(entryId)
        if (entry) {
            // Dispatch custom event for modal to handle
            const event = new CustomEvent("calendar:entry-click", {
                detail: { entry, tile },
            })
            document.dispatchEvent(event)
        }
    }

    async function handleTaskCheckbox(e: Event) {
        const target = e.target as HTMLInputElement
        if (!target.classList.contains("task-checkbox")) return

        const entryId = target.dataset.entryId
        if (!entryId) return

        try {
            await toggleTaskCompletion(config, entryId, target.checked)

            // Update local cache
            updateEntryInCache(entryId, { completed: target.checked })

            // Update visual state
            const tile = target.closest(".entry-tile")
            if (tile) {
                tile.classList.toggle("completed", target.checked)
            }
        } catch (_error) {
            // Revert checkbox
            target.checked = !target.checked
            showError(elements.container, "Failed to update task")
        }
    }

    function findEntryById(entryId: string): ServerEntry | undefined {
        for (const entries of state.entryCache.values()) {
            const found = entries.find((e) => e.id === entryId)
            if (found) return found
        }
        return undefined
    }

    function updateEntryInCache(entryId: string, updates: Partial<ServerEntry>): void {
        for (const [dateKey, entries] of state.entryCache.entries()) {
            const index = entries.findIndex((e) => e.id === entryId)
            if (index !== -1) {
                const updatedEntries = [...entries]
                updatedEntries[index] = { ...entries[index], ...updates }
                state.entryCache.set(dateKey, updatedEntries)
                break
            }
        }
    }

    async function navigateDays(offset: number) {
        const newCenterDate = addDays(state.centerDate, offset)
        state = { ...state, centerDate: newCenterDate }
        await loadAndRender()
    }

    async function goToToday() {
        state = { ...state, centerDate: new Date() }
        await loadAndRender()
    }

    async function goToDate(date: Date) {
        state = { ...state, centerDate: date }
        await loadAndRender()
    }

    async function refresh() {
        // Clear cache and reload
        state = { ...state, entryCache: new Map() }
        await loadAndRender()
    }

    async function loadAndRender() {
        const { start, end } = getRequiredDateRange(state.centerDate, state.visibleDays, 7)

        // Get dates that need to be fetched
        const allDateKeys: string[] = []
        let current = new Date(start)
        const endDate = new Date(end)
        while (current <= endDate) {
            allDateKeys.push(formatDateKey(current))
            current = addDays(current, 1)
        }

        const missingKeys = getMissingDateKeys(state.entryCache, allDateKeys)

        if (missingKeys.length > 0) {
            try {
                state = { ...state, isLoading: true }
                showLoading(elements.container)

                const fetchStart = missingKeys[0]
                const fetchEnd = missingKeys[missingKeys.length - 1]
                const newDays = await fetchEntries(config, fetchStart, fetchEnd)

                state = {
                    ...state,
                    entryCache: mergeEntryCache(state.entryCache, newDays),
                    isLoading: false,
                }

                hideLoading(elements.container)
            } catch (_error) {
                state = { ...state, isLoading: false }
                hideLoading(elements.container)
                showError(elements.container, "Failed to load calendar entries")
            }
        }

        renderCurrentView()
    }

    function renderCurrentView() {
        // Get dates to display
        const halfDays = Math.floor(state.visibleDays / 2)
        const visibleDates = getDatesAround(state.centerDate, halfDays, halfDays)

        // Render grid
        renderGrid(elements.grid, visibleDates, state.entryCache)

        // Update header
        updateMonthLabel(elements.monthLabel, state.centerDate)
    }

    function destroy() {
        // Clean up event listeners
        document.removeEventListener("keydown", handleKeydown)
        elements.grid.removeEventListener("click", handleEntryClick)
        elements.grid.removeEventListener("change", handleTaskCheckbox)
    }
}
