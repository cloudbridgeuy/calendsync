/**
 * Hook for managing UI annotations in dev mode.
 * Handles fetching, creating, deleting, resolving, and real-time sync via SSE.
 */

import { useCallback, useEffect, useRef, useState } from "react"
import {
  type Annotation,
  type AnnotationsListResponse,
  formatAnnotationsMarkdown,
  isVisibleAnnotation,
} from "../../core/calendar/annotations"

const API_BASE = "/_dev/annotations"
const SSE_URL = "/_dev/annotations/events"
const MAX_RECONNECT_DELAY = 10_000

export function useAnnotations() {
  const [annotations, setAnnotations] = useState<Annotation[]>([])
  const [isActive, setIsActive] = useState(false)
  const [selectedAnnotation, setSelectedAnnotation] = useState<Annotation | null>(null)
  const reconnectDelay = useRef(1000)

  // SSE real-time sync
  useEffect(() => {
    let eventSource: EventSource | null = null
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null
    let disposed = false

    function connect() {
      if (disposed) return

      eventSource = new EventSource(SSE_URL)

      eventSource.onopen = () => {
        reconnectDelay.current = 1000
      }

      eventSource.addEventListener("annotation.created", (event: MessageEvent<string>) => {
        const annotation: Annotation = JSON.parse(event.data)
        setAnnotations((prev) => [annotation, ...prev])
      })

      eventSource.addEventListener("annotation.updated", (event: MessageEvent<string>) => {
        const annotation: Annotation = JSON.parse(event.data)
        setAnnotations((prev) => prev.map((a) => (a.id === annotation.id ? annotation : a)))
        setSelectedAnnotation((prev) => {
          if (prev?.id !== annotation.id) return prev
          return isVisibleAnnotation(annotation) ? annotation : null
        })
      })

      eventSource.addEventListener("annotation.deleted", (event: MessageEvent<string>) => {
        const annotation: Annotation = JSON.parse(event.data)
        setAnnotations((prev) => prev.filter((a) => a.id !== annotation.id))
        setSelectedAnnotation((prev) => (prev?.id === annotation.id ? null : prev))
      })

      eventSource.onerror = () => {
        eventSource?.close()
        if (disposed) return

        const delay = reconnectDelay.current
        reconnectDelay.current = Math.min(delay * 2, MAX_RECONNECT_DELAY)
        reconnectTimer = setTimeout(connect, delay)
      }
    }

    connect()

    return () => {
      disposed = true
      eventSource?.close()
      if (reconnectTimer !== null) {
        clearTimeout(reconnectTimer)
      }
    }
  }, [])

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

  const create = useCallback(async (data: Record<string, unknown>) => {
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
  }, [])

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

  const acknowledge = useCallback(async (id: string) => {
    try {
      const res = await fetch(`${API_BASE}/${id}/acknowledge`, { method: "PATCH" })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
    } catch (err) {
      console.error("[Annotations] Failed to acknowledge:", err)
    }
  }, [])

  const resolve = useCallback(async (id: string, summary: string) => {
    try {
      const res = await fetch(`${API_BASE}/${id}/resolve`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ summary }),
      })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
    } catch (err) {
      console.error("[Annotations] Failed to resolve:", err)
    }
  }, [])

  const dismiss = useCallback(async (id: string, reason: string) => {
    try {
      const res = await fetch(`${API_BASE}/${id}/dismiss`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ reason }),
      })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
    } catch (err) {
      console.error("[Annotations] Failed to dismiss:", err)
    }
  }, [])

  const reply = useCallback(async (id: string, message: string) => {
    try {
      const res = await fetch(`${API_BASE}/${id}/thread`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ message, author: "human" }),
      })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
    } catch (err) {
      console.error("[Annotations] Failed to reply:", err)
    }
  }, [])

  const selectAnnotation = useCallback((ann: Annotation) => {
    setSelectedAnnotation(ann)
  }, [])

  const deselectAnnotation = useCallback(() => {
    setSelectedAnnotation(null)
  }, [])

  return {
    annotations,
    isActive,
    selectedAnnotation,
    toggle,
    create,
    remove,
    clearAll,
    copyToClipboard,
    acknowledge,
    resolve,
    dismiss,
    reply,
    selectAnnotation,
    deselectAnnotation,
  }
}
