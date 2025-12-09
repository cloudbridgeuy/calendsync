/**
 * Hook for managing entry form state.
 * Extracts form logic from EntryModal for separation of concerns.
 */

import { createDefaultFormData, entryToFormData, validateFormData } from "@core/calendar"
import type { ServerEntry } from "@core/calendar/types"
import { useCallback, useEffect, useState } from "react"
import type { EntryFormData } from "../types"

/** Options for useEntryForm hook */
export interface UseEntryFormOptions {
  /** Modal mode: create new entry or edit existing */
  mode: "create" | "edit"
  /** Entry data for edit mode */
  entry?: ServerEntry
  /** Default date for create mode */
  defaultDate?: string
}

/** Return type for useEntryForm hook */
export interface UseEntryFormReturn {
  /** Current form data */
  formData: EntryFormData
  /** Validation errors from last validation */
  validationErrors: string[]
  /** Handle form field change */
  handleChange: (field: keyof EntryFormData, value: string | boolean) => void
  /** Validate form data, returns true if valid */
  validate: () => boolean
  /** Clear validation errors */
  clearErrors: () => void
}

/**
 * Hook to manage entry form state and validation.
 */
export function useEntryForm(options: UseEntryFormOptions): UseEntryFormReturn {
  const { mode, entry, defaultDate } = options

  // Initialize form data based on mode
  const [formData, setFormData] = useState<EntryFormData>(() => {
    if (mode === "edit" && entry) {
      return entryToFormData(entry)
    }
    return createDefaultFormData(defaultDate)
  })

  const [validationErrors, setValidationErrors] = useState<string[]>([])

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
    setValidationErrors([])
  }, [])

  /**
   * Validate form data.
   */
  const validate = useCallback((): boolean => {
    const result = validateFormData(formData)
    if (!result.valid) {
      setValidationErrors(result.errors)
      return false
    }
    setValidationErrors([])
    return true
  }, [formData])

  /**
   * Clear validation errors.
   */
  const clearErrors = useCallback(() => {
    setValidationErrors([])
  }, [])

  return {
    formData,
    validationErrors,
    handleChange,
    validate,
    clearErrors,
  }
}
