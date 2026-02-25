/**
 * Main annotation overlay component.
 * When active, highlights hovered elements and captures annotations on click.
 */

import { useCallback, useEffect, useRef, useState } from "react"
import {
  type Annotation,
  type BoundingBox,
  buildCreateAnnotationBody,
  extractComputedStyles,
  generateSelector,
} from "../../core/calendar/annotations"
import { AnnotationMarker } from "./AnnotationMarker"
import { AnnotationNotePopup } from "./AnnotationNotePopup"

interface AnnotationOverlayProps {
  annotations: Annotation[]
  isActive: boolean
  onCreate: (
    data: Omit<Annotation, "id" | "timestamp" | "resolved" | "resolution_summary">,
  ) => Promise<void>
  onRemove: (id: string) => void
}

/** Get parent selectors for an element (up to 3 levels). */
function getParentSelectors(element: Element): string[] {
  const selectors: string[] = []
  let current = element.parentElement
  let depth = 0
  while (current && current !== document.body && depth < 3) {
    const tag = current.tagName.toLowerCase()
    if (current.id) {
      selectors.unshift(`${tag}#${current.id}`)
    } else if (current.classList.length > 0) {
      selectors.unshift(`${tag}.${Array.from(current.classList).join(".")}`)
    } else {
      selectors.unshift(tag)
    }
    current = current.parentElement
    depth++
  }
  return selectors
}

/** Try to find the React component name from the fiber. */
function getReactComponentName(element: Element): string | null {
  const fiberKey = Object.keys(element).find((key) => key.startsWith("__reactFiber$"))
  if (!fiberKey) return null

  // biome-ignore lint/suspicious/noExplicitAny: React internal fiber access
  let fiber = (element as any)[fiberKey]
  while (fiber) {
    if (fiber.type && typeof fiber.type === "function") {
      return fiber.type.displayName || fiber.type.name || null
    }
    fiber = fiber.return
  }
  return null
}

export function AnnotationOverlay({
  annotations,
  isActive,
  onCreate,
  onRemove,
}: AnnotationOverlayProps) {
  const [hoveredRect, setHoveredRect] = useState<DOMRect | null>(null)
  const [hoveredInfo, setHoveredInfo] = useState<string>("")
  const [notePopup, setNotePopup] = useState<{
    element: Element
    position: { top: number; left: number }
  } | null>(null)
  const overlayRef = useRef<HTMLDivElement>(null)

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!isActive || notePopup) return

      const target = e.target as Element
      // Skip our own overlay elements
      if (
        target.closest(".annotation-overlay, .annotation-marker, .annotation-note-popup, .dev-menu")
      ) {
        setHoveredRect(null)
        return
      }

      const rect = target.getBoundingClientRect()
      setHoveredRect(rect)

      const tag = target.tagName.toLowerCase()
      const component = getReactComponentName(target)
      const info = component ? `<${tag}> (${component})` : `<${tag}>`
      setHoveredInfo(info)
    },
    [isActive, notePopup],
  )

  const handleClick = useCallback(
    (e: MouseEvent) => {
      if (!isActive || notePopup) return

      const target = e.target as Element
      if (
        target.closest(".annotation-overlay, .annotation-marker, .annotation-note-popup, .dev-menu")
      ) {
        return
      }

      e.preventDefault()
      e.stopPropagation()

      const rect = target.getBoundingClientRect()
      setNotePopup({
        element: target,
        position: { top: rect.bottom + 8, left: rect.left },
      })
    },
    [isActive, notePopup],
  )

  useEffect(() => {
    if (!isActive) {
      setHoveredRect(null)
      setNotePopup(null)
      return
    }

    document.addEventListener("mousemove", handleMouseMove, true)
    document.addEventListener("click", handleClick, true)
    return () => {
      document.removeEventListener("mousemove", handleMouseMove, true)
      document.removeEventListener("click", handleClick, true)
    }
  }, [isActive, handleMouseMove, handleClick])

  const handleSaveNote = useCallback(
    async (note: string) => {
      if (!notePopup) return

      const el = notePopup.element
      const rect = el.getBoundingClientRect()

      const parentSelectors = getParentSelectors(el)
      const selector = generateSelector(
        el.tagName,
        Array.from(el.classList),
        el.id || null,
        parentSelectors,
      )

      const styles = window.getComputedStyle(el)
      const computedStyles = extractComputedStyles({
        color: styles.color,
        backgroundColor: styles.backgroundColor,
        fontSize: styles.fontSize,
        fontFamily: styles.fontFamily,
        padding: styles.padding,
        margin: styles.margin,
        width: styles.width,
        height: styles.height,
        display: styles.display,
        position: styles.position,
      })

      const boundingBox: BoundingBox = {
        top: rect.top,
        left: rect.left,
        width: rect.width,
        height: rect.height,
      }

      const body = buildCreateAnnotationBody(
        selector,
        getReactComponentName(el),
        el.tagName.toLowerCase(),
        el.textContent ?? "",
        note,
        boundingBox,
        computedStyles,
        null, // skip screenshot for now
      )

      await onCreate(body)
      setNotePopup(null)
    },
    [notePopup, onCreate],
  )

  const handleMarkerClick = useCallback(
    (annotation: Annotation) => {
      if (window.confirm(`Remove annotation?\n\n"${annotation.note}"`)) {
        onRemove(annotation.id)
      }
    },
    [onRemove],
  )

  return (
    <>
      {/* Highlight overlay */}
      {isActive && hoveredRect && (
        <div className="annotation-overlay">
          <div
            className="annotation-highlight"
            style={{
              position: "fixed",
              top: hoveredRect.top,
              left: hoveredRect.left,
              width: hoveredRect.width,
              height: hoveredRect.height,
            }}
          />
          <div
            className="annotation-info"
            style={{
              position: "fixed",
              top: hoveredRect.top - 28,
              left: hoveredRect.left,
            }}
          >
            {hoveredInfo}
          </div>
        </div>
      )}

      {/* Markers on annotated elements */}
      {annotations.map((annotation, i) => (
        <AnnotationMarker
          key={annotation.id}
          annotation={annotation}
          index={i}
          onClick={handleMarkerClick}
        />
      ))}

      {/* Note input popup */}
      {notePopup && (
        <AnnotationNotePopup
          position={notePopup.position}
          onSave={handleSaveNote}
          onCancel={() => setNotePopup(null)}
        />
      )}

      {/* Active mode indicator */}
      {isActive && (
        <div ref={overlayRef} className="annotation-active-badge">
          Annotating — click any element
        </div>
      )}
    </>
  )
}
