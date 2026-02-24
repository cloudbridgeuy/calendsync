/**
 * Numbered circle marker positioned on an annotated element.
 */

import type { Annotation } from "../../core/calendar/annotations"

interface AnnotationMarkerProps {
  annotation: Annotation
  index: number
  onClick: (annotation: Annotation) => void
}

export function AnnotationMarker({ annotation, index, onClick }: AnnotationMarkerProps) {
  const { bounding_box: box } = annotation

  return (
    <button
      type="button"
      className={`annotation-marker ${annotation.resolved ? "annotation-marker--resolved" : ""}`}
      style={{
        position: "fixed",
        top: box.top - 12,
        left: box.left + box.width - 12,
      }}
      onClick={(e) => {
        e.stopPropagation()
        onClick(annotation)
      }}
      title={annotation.note}
    >
      {index + 1}
    </button>
  )
}
