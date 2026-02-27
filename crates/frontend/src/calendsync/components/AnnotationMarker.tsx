/**
 * Numbered circle marker positioned on an annotated element.
 * Color-coded by annotation status.
 */

import type { Annotation } from "../../core/calendar/annotations"
import { statusColor } from "../../core/calendar/annotations"

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
      className="annotation-marker"
      style={{
        position: "fixed",
        top: box.top - 12,
        left: box.left + box.width - 12,
        backgroundColor: statusColor(annotation.status),
      }}
      onClick={(e) => {
        e.stopPropagation()
        onClick(annotation)
      }}
      title={annotation.comment}
    >
      {index + 1}
    </button>
  )
}
