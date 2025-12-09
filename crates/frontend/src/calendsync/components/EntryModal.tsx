/**
 * Entry modal component for creating and editing calendar entries.
 */

import type { ServerEntry } from "@core/calendar/types"
import { useCallback, useState } from "react"
import { useEntryApi, useEntryForm, useFocusTrap } from "../hooks"

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

  // Form state from extracted hook
  const { formData, validationErrors, handleChange, validate } = useEntryForm({
    mode,
    entry,
    defaultDate,
  })

  // Submission state
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // API hook
  const api = useEntryApi({ calendarId })

  // Focus trap for accessibility (traps Tab navigation within modal)
  const { containerRef: modalRef } = useFocusTrap({
    isActive: !isSubmitting,
    onEscape: onClose,
    autoFocusId: "entry-title",
  })

  /**
   * Handle form field change wrapper that also clears API errors.
   */
  const handleFieldChange = useCallback(
    (field: Parameters<typeof handleChange>[0], value: Parameters<typeof handleChange>[1]) => {
      handleChange(field, value)
      setError(null)
    },
    [handleChange],
  )

  /**
   * Handle form submission.
   */
  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault()

      // Validate form data
      if (!validate()) {
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
    [formData, mode, entry, api, onSave, validate],
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
              onChange={(e) => handleFieldChange("title", e.target.value)}
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
              onChange={(e) => handleFieldChange("date", e.target.value)}
              disabled={isSubmitting}
            />
          </div>

          {/* Entry Type Selector */}
          <div className="form-group">
            <label htmlFor="entry-type">Type</label>
            <select
              id="entry-type"
              value={formData.entryType}
              onChange={(e) => handleFieldChange("entryType", e.target.value)}
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
                  onChange={(e) => handleFieldChange("startTime", e.target.value)}
                  disabled={isSubmitting}
                />
              </div>
              <div className="form-group">
                <label htmlFor="entry-end-time">End</label>
                <input
                  id="entry-end-time"
                  type="time"
                  value={formData.endTime || ""}
                  onChange={(e) => handleFieldChange("endTime", e.target.value)}
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
                onChange={(e) => handleFieldChange("endDate", e.target.value)}
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
                  onChange={(e) => handleFieldChange("completed", e.target.checked)}
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
              onChange={(e) => handleFieldChange("description", e.target.value)}
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
              onChange={(e) => handleFieldChange("location", e.target.value)}
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
              <button type="submit" className="btn btn-primary" disabled={isSubmitting}>
                {isSubmitting ? "Saving..." : mode === "create" ? "Create" : "Save"}
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  )
}
