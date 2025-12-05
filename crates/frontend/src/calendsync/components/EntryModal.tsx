/**
 * Entry modal component for creating and editing calendar entries.
 */

import { createDefaultFormData, entryToFormData, validateFormData } from "@core/calendar"
import type { ServerEntry } from "@core/calendar/types"
import { useCallback, useEffect, useState } from "react"
import { useEntryApi, useFocusTrap } from "../hooks"
import type { EntryFormData } from "../types"

/**
 * Props for the EntryModal component.
 */
export interface EntryModalProps {
    /** Modal mode: create new entry or edit existing */
    mode: "create" | "edit"
    /** Pre-fetched entry data for edit mode (from SSR or cache) */
    entry?: ServerEntry
    /** Default date for create mode (pre-fill the date field) */
    defaultDate?: string
    /** Calendar ID for API calls */
    calendarId: string
    /** Callback when the modal is closed (cancel or save) */
    onClose: () => void
    /** Callback after successfully saving an entry */
    onSave: (entry: ServerEntry) => void
    /** Callback after successfully deleting an entry */
    onDelete?: (entryId: string) => void
}

/**
 * Modal component for creating/editing calendar entries.
 */
export function EntryModal(props: EntryModalProps) {
    const { mode, entry, defaultDate, calendarId, onClose, onSave, onDelete } = props

    // Initialize form data based on mode
    const [formData, setFormData] = useState<EntryFormData>(() => {
        if (mode === "edit" && entry) {
            return entryToFormData(entry)
        }
        return createDefaultFormData(defaultDate)
    })

    // Form state
    const [isSubmitting, setIsSubmitting] = useState(false)
    const [error, setError] = useState<string | null>(null)
    const [validationErrors, setValidationErrors] = useState<string[]>([])

    // API hook
    const api = useEntryApi({ calendarId })

    // Focus trap for accessibility (traps Tab navigation within modal)
    const { containerRef: modalRef } = useFocusTrap({
        isActive: !isSubmitting,
        onEscape: onClose,
        autoFocusId: "entry-title",
    })

    // Update form data if entry prop changes (for client-side fetch)
    useEffect(() => {
        if (mode === "edit" && entry) {
            setFormData(entryToFormData(entry))
        }
    }, [mode, entry])

    /**
     * Handle form field changes.
     */
    const handleChange = useCallback((field: keyof EntryFormData, value: string | boolean) => {
        setFormData((prev) => {
            const updated = { ...prev, [field]: value }

            // Handle entry type changes
            if (field === "entryType") {
                switch (value) {
                    case "all_day":
                        updated.isAllDay = true
                        updated.startTime = undefined
                        updated.endTime = undefined
                        updated.endDate = undefined
                        updated.completed = undefined
                        break
                    case "timed":
                        updated.isAllDay = false
                        updated.endDate = undefined
                        updated.completed = undefined
                        break
                    case "multi_day":
                        updated.isAllDay = false
                        updated.startTime = undefined
                        updated.endTime = undefined
                        updated.completed = undefined
                        break
                    case "task":
                        updated.isAllDay = false
                        updated.startTime = undefined
                        updated.endTime = undefined
                        updated.endDate = undefined
                        // Preserve existing completed value or default to false
                        if (updated.completed === undefined) {
                            updated.completed = false
                        }
                        break
                }
            }

            return updated
        })

        // Clear errors when user makes changes
        setError(null)
        setValidationErrors([])
    }, [])

    /**
     * Handle form submission.
     */
    const handleSubmit = useCallback(
        async (e: React.FormEvent) => {
            e.preventDefault()

            // Validate form data
            const validation = validateFormData(formData)
            if (!validation.valid) {
                setValidationErrors(validation.errors)
                return
            }

            setIsSubmitting(true)
            setError(null)

            try {
                let result: ServerEntry
                if (mode === "create") {
                    result = await api.createEntry(formData)
                } else if (entry) {
                    result = await api.updateEntry(entry.id, formData)
                } else {
                    throw new Error("Cannot update: no entry ID")
                }

                onSave(result)
            } catch (err) {
                setError(err instanceof Error ? err.message : "Failed to save entry")
            } finally {
                setIsSubmitting(false)
            }
        },
        [formData, mode, entry, api, onSave],
    )

    /**
     * Handle entry deletion.
     */
    const handleDelete = useCallback(async () => {
        if (!entry) return

        const confirmed = window.confirm(`Are you sure you want to delete "${entry.title}"?`)
        if (!confirmed) return

        setIsSubmitting(true)
        setError(null)

        try {
            await api.deleteEntry(entry.id)
            onDelete?.(entry.id)
            onClose()
        } catch (err) {
            setError(err instanceof Error ? err.message : "Failed to delete entry")
            setIsSubmitting(false)
        }
    }, [entry, api, onDelete, onClose])

    // Note: Escape key handling is now done by useFocusTrap

    /**
     * Handle overlay click to close modal.
     */
    const handleOverlayClick = useCallback(
        (e: React.MouseEvent) => {
            if (e.target === e.currentTarget && !isSubmitting) {
                onClose()
            }
        },
        [onClose, isSubmitting],
    )

    return (
        // biome-ignore lint/a11y/noStaticElementInteractions: Modal overlay click-to-close is intentional
        <div className="modal-overlay" onClick={handleOverlayClick} role="presentation">
            <div
                ref={modalRef}
                className="modal"
                onClick={(e) => e.stopPropagation()}
                onKeyDown={(e) => e.stopPropagation()}
                role="dialog"
                aria-modal="true"
                aria-labelledby="modal-title"
            >
                <h2 id="modal-title" className="modal-title">
                    {mode === "create" ? "New Entry" : "Edit Entry"}
                </h2>

                <form onSubmit={handleSubmit} className="modal-form">
                    {/* Title */}
                    <div className="form-group">
                        <label htmlFor="entry-title">Title</label>
                        <input
                            id="entry-title"
                            type="text"
                            value={formData.title}
                            onChange={(e) => handleChange("title", e.target.value)}
                            placeholder="Event title"
                            disabled={isSubmitting}
                        />
                    </div>

                    {/* Date */}
                    <div className="form-group">
                        <label htmlFor="entry-date">
                            {formData.entryType === "multi_day" ? "Start Date" : "Date"}
                        </label>
                        <input
                            id="entry-date"
                            type="date"
                            value={formData.date}
                            onChange={(e) => handleChange("date", e.target.value)}
                            disabled={isSubmitting}
                        />
                    </div>

                    {/* Entry Type Selector */}
                    <div className="form-group">
                        <label htmlFor="entry-type">Type</label>
                        <select
                            id="entry-type"
                            value={formData.entryType}
                            onChange={(e) => handleChange("entryType", e.target.value)}
                            disabled={isSubmitting}
                        >
                            <option value="all_day">All Day</option>
                            <option value="timed">Timed</option>
                            <option value="multi_day">Multi-Day</option>
                            <option value="task">Task</option>
                        </select>
                    </div>

                    {/* Time inputs (only for timed entries) */}
                    {formData.entryType === "timed" && (
                        <div className="form-row time-inputs">
                            <div className="form-group">
                                <label htmlFor="entry-start-time">Start</label>
                                <input
                                    id="entry-start-time"
                                    type="time"
                                    value={formData.startTime || ""}
                                    onChange={(e) => handleChange("startTime", e.target.value)}
                                    disabled={isSubmitting}
                                />
                            </div>
                            <div className="form-group">
                                <label htmlFor="entry-end-time">End</label>
                                <input
                                    id="entry-end-time"
                                    type="time"
                                    value={formData.endTime || ""}
                                    onChange={(e) => handleChange("endTime", e.target.value)}
                                    disabled={isSubmitting}
                                />
                            </div>
                        </div>
                    )}

                    {/* End Date (for multi-day entries) */}
                    {formData.entryType === "multi_day" && (
                        <div className="form-group">
                            <label htmlFor="entry-end-date">End Date</label>
                            <input
                                id="entry-end-date"
                                type="date"
                                value={formData.endDate || ""}
                                min={formData.date}
                                onChange={(e) => handleChange("endDate", e.target.value)}
                                disabled={isSubmitting}
                            />
                        </div>
                    )}

                    {/* Completed checkbox (for task entries) */}
                    {formData.entryType === "task" && (
                        <div className="form-group form-checkbox">
                            <label>
                                <input
                                    type="checkbox"
                                    checked={formData.completed || false}
                                    onChange={(e) => handleChange("completed", e.target.checked)}
                                    disabled={isSubmitting}
                                />
                                Completed
                            </label>
                        </div>
                    )}

                    {/* Description */}
                    <div className="form-group">
                        <label htmlFor="entry-description">Description</label>
                        <textarea
                            id="entry-description"
                            value={formData.description || ""}
                            onChange={(e) => handleChange("description", e.target.value)}
                            placeholder="Add a description..."
                            rows={3}
                            disabled={isSubmitting}
                        />
                    </div>

                    {/* Location */}
                    <div className="form-group">
                        <label htmlFor="entry-location">Location</label>
                        <input
                            id="entry-location"
                            type="text"
                            value={formData.location || ""}
                            onChange={(e) => handleChange("location", e.target.value)}
                            placeholder="Add a location..."
                            disabled={isSubmitting}
                        />
                    </div>

                    {/* Validation errors */}
                    {validationErrors.length > 0 && (
                        <div className="form-errors">
                            {validationErrors.map((err) => (
                                <div key={err} className="form-error">
                                    {err}
                                </div>
                            ))}
                        </div>
                    )}

                    {/* API error */}
                    {error && <div className="form-error api-error">{error}</div>}

                    {/* Actions */}
                    <div className="modal-actions">
                        {mode === "edit" && (
                            <button
                                type="button"
                                className="btn btn-danger"
                                onClick={handleDelete}
                                disabled={isSubmitting}
                            >
                                Delete
                            </button>
                        )}
                        <div className="modal-actions-right">
                            <button
                                type="button"
                                className="btn btn-secondary"
                                onClick={onClose}
                                disabled={isSubmitting}
                            >
                                Cancel
                            </button>
                            <button
                                type="submit"
                                className="btn btn-primary"
                                disabled={isSubmitting}
                            >
                                {isSubmitting ? "Saving..." : mode === "create" ? "Create" : "Save"}
                            </button>
                        </div>
                    </div>
                </form>
            </div>
        </div>
    )
}
