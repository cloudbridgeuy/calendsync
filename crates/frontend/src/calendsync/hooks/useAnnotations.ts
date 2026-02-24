/**
 * Hook for managing UI annotations in dev mode.
 * Handles fetching, creating, deleting, and resolving annotations via the dev API.
 */

import { useCallback, useEffect, useState } from "react"
import type { Annotation, AnnotationsListResponse } from "../../core/calendar/annotations"
import { formatAnnotationsMarkdown } from "../../core/calendar/annotations"

const API_BASE = "/_dev/annotations"

export function useAnnotations() {
  const [annotations, setAnnotations] = useState<Annotation[]>([])
  const [isActive, setIsActive] = useState(false)

  // Fetch all annotations on mount
  useEffect(() => {
    fetch(API_BASE)
      .then((res) => res.json() as Promise<AnnotationsListResponse>)
      .then((data) => setAnnotations(data.annotations))
      .catch((err) => console.error("[Annotations] Failed to fetch:", err))
  }, [])

  const toggle = useCallback(() => {
    setIsActive((prev) => !prev)
  }, [])

  const create = useCallback(
    async (data: Omit<Annotation, "id" | "timestamp" | "resolved" | "resolution_summary">) => {
      try {
        const res = await fetch(API_BASE, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(data),
        })
        if (!res.ok) throw new Error(`HTTP ${res.status}`)
        // Refetch to get the server-assigned id/timestamp
        const listRes = await fetch(API_BASE)
        const listData = (await listRes.json()) as AnnotationsListResponse
        setAnnotations(listData.annotations)
      } catch (err) {
        console.error("[Annotations] Failed to create:", err)
      }
    },
    [],
  )

  const remove = useCallback(async (id: string) => {
    try {
      await fetch(`${API_BASE}/${id}`, { method: "DELETE" })
      setAnnotations((prev) => prev.filter((a) => a.id !== id))
    } catch (err) {
      console.error("[Annotations] Failed to delete:", err)
    }
  }, [])

  const clearAll = useCallback(async () => {
    try {
      await fetch(API_BASE, { method: "DELETE" })
      setAnnotations([])
    } catch (err) {
      console.error("[Annotations] Failed to clear:", err)
    }
  }, [])

  const copyToClipboard = useCallback(async () => {
    const markdown = formatAnnotationsMarkdown(annotations)
    try {
      await navigator.clipboard.writeText(markdown)
    } catch {
      console.error("[Annotations] Failed to copy to clipboard")
    }
  }, [annotations])

  return {
    annotations,
    isActive,
    toggle,
    create,
    remove,
    clearAll,
    copyToClipboard,
  }
}
