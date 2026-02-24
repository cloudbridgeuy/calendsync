/**
 * Client-only wrapper that connects annotation overlay + DevMenu annotation props.
 * Only rendered when devMode is true.
 */

import { useAnnotations } from "../hooks/useAnnotations"
import type { InitialData } from "../types"
import { AnnotationOverlay } from "./AnnotationOverlay"
import { DevMenu } from "./DevMenu"

interface DevAnnotationLayerProps {
  initialData: InitialData
}

export function DevAnnotationLayer({ initialData }: DevAnnotationLayerProps) {
  const { annotations, isActive, toggle, create, remove, clearAll, copyToClipboard } =
    useAnnotations()

  return (
    <>
      <DevMenu
        initialData={initialData}
        onToggleAnnotations={toggle}
        onClearAnnotations={clearAll}
        onCopyAnnotations={copyToClipboard}
        annotationCount={annotations.length}
        isAnnotating={isActive}
      />
      <AnnotationOverlay
        annotations={annotations}
        isActive={isActive}
        onCreate={create}
        onRemove={remove}
      />
    </>
  )
}
