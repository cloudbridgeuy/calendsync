/**
 * Entry modal for creating and editing calendar entries.
 * This is the Imperative Shell - handles modal UI interactions.
 */

import { formatDateKey } from "../core/calendar/dates"
import type { ServerEntry } from "../core/calendar/types"
import type { ApiConfig } from "./api"
import { createEntry, deleteEntry, updateEntry } from "./api"

/**
 * Modal DOM elements.
 */
export interface ModalElements {
    overlay: HTMLElement
    modal: HTMLElement
    form: HTMLFormElement
    titleInput: HTMLInputElement
    dateInput: HTMLInputElement
    startTimeInput: HTMLInputElement
    endTimeInput: HTMLInputElement
    allDayCheckbox: HTMLInputElement
    descriptionInput: HTMLTextAreaElement
    locationInput: HTMLInputElement
    deleteButton: HTMLButtonElement
    cancelButton: HTMLButtonElement
    submitButton: HTMLButtonElement
    modalTitle: HTMLElement
}

/**
 * Modal state.
 */
interface ModalState {
    isOpen: boolean
    mode: "create" | "edit"
    entry: ServerEntry | null
}

/**
 * Get modal DOM elements.
 */
export function getModalElements(): ModalElements | null {
    const overlay = document.getElementById("entry-modal-overlay")
    const modal = document.getElementById("entry-modal")
    const form = document.getElementById("entry-form") as HTMLFormElement
    const titleInput = document.getElementById("entry-title") as HTMLInputElement
    const dateInput = document.getElementById("entry-date") as HTMLInputElement
    const startTimeInput = document.getElementById("entry-start-time") as HTMLInputElement
    const endTimeInput = document.getElementById("entry-end-time") as HTMLInputElement
    const allDayCheckbox = document.getElementById("entry-all-day") as HTMLInputElement
    const descriptionInput = document.getElementById("entry-description") as HTMLTextAreaElement
    const locationInput = document.getElementById("entry-location") as HTMLInputElement
    const deleteButton = document.getElementById("entry-delete") as HTMLButtonElement
    const cancelButton = document.getElementById("entry-cancel") as HTMLButtonElement
    const submitButton = document.getElementById("entry-submit") as HTMLButtonElement
    const modalTitle = document.getElementById("modal-title")

    if (
        !overlay ||
        !modal ||
        !form ||
        !titleInput ||
        !dateInput ||
        !startTimeInput ||
        !endTimeInput ||
        !allDayCheckbox ||
        !descriptionInput ||
        !locationInput ||
        !deleteButton ||
        !cancelButton ||
        !submitButton ||
        !modalTitle
    ) {
        return null
    }

    return {
        overlay,
        modal,
        form,
        titleInput,
        dateInput,
        startTimeInput,
        endTimeInput,
        allDayCheckbox,
        descriptionInput,
        locationInput,
        deleteButton,
        cancelButton,
        submitButton,
        modalTitle,
    }
}

/**
 * Create the entry modal controller.
 */
export function createEntryModalController(
    elements: ModalElements,
    config: ApiConfig,
    onSave: () => void,
) {
    let state: ModalState = {
        isOpen: false,
        mode: "create",
        entry: null,
    }

    // Public API
    const controller = {
        openCreate,
        openEdit,
        close,
        isOpen: () => state.isOpen,
    }

    // Initialize
    init()

    return controller

    // --- Implementation ---

    function init() {
        // Form submission
        elements.form.addEventListener("submit", handleSubmit)

        // Cancel button
        elements.cancelButton.addEventListener("click", close)

        // Delete button
        elements.deleteButton.addEventListener("click", handleDelete)

        // Close on overlay click
        elements.overlay.addEventListener("click", (e) => {
            if (e.target === elements.overlay) {
                close()
            }
        })

        // Close on Escape
        document.addEventListener("keydown", (e) => {
            if (e.key === "Escape" && state.isOpen) {
                close()
            }
        })

        // Toggle time inputs based on all-day checkbox
        elements.allDayCheckbox.addEventListener("change", () => {
            toggleTimeInputs(!elements.allDayCheckbox.checked)
        })

        // Listen for entry click events from the calendar
        document.addEventListener("calendar:entry-click", ((e: CustomEvent) => {
            openEdit(e.detail.entry)
        }) as EventListener)
    }

    function openCreate(defaultDate?: string) {
        state = {
            isOpen: true,
            mode: "create",
            entry: null,
        }

        // Reset form
        elements.form.reset()

        // Set default date
        const dateStr = defaultDate || formatDateKey(new Date())
        elements.dateInput.value = dateStr

        // Update UI
        elements.modalTitle.textContent = "New Entry"
        elements.deleteButton.style.display = "none"
        elements.submitButton.textContent = "Create"

        toggleTimeInputs(true)
        show()

        // Focus title input
        setTimeout(() => elements.titleInput.focus(), 100)
    }

    function openEdit(entry: ServerEntry) {
        state = {
            isOpen: true,
            mode: "edit",
            entry,
        }

        // Populate form
        elements.titleInput.value = entry.title
        elements.dateInput.value = entry.date
        elements.startTimeInput.value = entry.startTime || ""
        elements.endTimeInput.value = entry.endTime || ""
        elements.allDayCheckbox.checked = entry.isAllDay
        elements.descriptionInput.value = entry.description || ""
        elements.locationInput.value = entry.location || ""

        // Update UI
        elements.modalTitle.textContent = "Edit Entry"
        elements.deleteButton.style.display = "block"
        elements.submitButton.textContent = "Save"

        toggleTimeInputs(!entry.isAllDay)
        show()

        // Focus title input
        setTimeout(() => elements.titleInput.focus(), 100)
    }

    function close() {
        state = { ...state, isOpen: false }
        hide()
    }

    function show() {
        elements.overlay.classList.add("visible")
        elements.modal.classList.add("visible")
        document.body.style.overflow = "hidden"
    }

    function hide() {
        elements.overlay.classList.remove("visible")
        elements.modal.classList.remove("visible")
        document.body.style.overflow = ""
    }

    function toggleTimeInputs(enabled: boolean) {
        elements.startTimeInput.disabled = !enabled
        elements.endTimeInput.disabled = !enabled

        const timeContainer = elements.startTimeInput.closest(".time-inputs")
        if (timeContainer) {
            timeContainer.classList.toggle("disabled", !enabled)
        }
    }

    async function handleSubmit(e: Event) {
        e.preventDefault()

        const formData = {
            title: elements.titleInput.value.trim(),
            date: elements.dateInput.value,
            startTime: elements.allDayCheckbox.checked
                ? undefined
                : elements.startTimeInput.value || undefined,
            endTime: elements.allDayCheckbox.checked
                ? undefined
                : elements.endTimeInput.value || undefined,
            isAllDay: elements.allDayCheckbox.checked,
            description: elements.descriptionInput.value.trim() || undefined,
            location: elements.locationInput.value.trim() || undefined,
        }

        if (!formData.title) {
            elements.titleInput.focus()
            return
        }

        try {
            elements.submitButton.disabled = true

            if (state.mode === "create") {
                await createEntry(config, formData)
            } else if (state.entry) {
                await updateEntry(config, state.entry.id, formData)
            }

            close()
            onSave()
        } catch (error) {
            console.error("Failed to save entry:", error)
            alert("Failed to save entry. Please try again.")
        } finally {
            elements.submitButton.disabled = false
        }
    }

    async function handleDelete() {
        if (!state.entry) return

        const confirmed = confirm(`Are you sure you want to delete "${state.entry.title}"?`)
        if (!confirmed) return

        try {
            elements.deleteButton.disabled = true
            await deleteEntry(config, state.entry.id)
            close()
            onSave()
        } catch (error) {
            console.error("Failed to delete entry:", error)
            alert("Failed to delete entry. Please try again.")
        } finally {
            elements.deleteButton.disabled = false
        }
    }
}
