/**
 * Client-only wrapper that connects annotation overlay + DevMenu annotation props.
 * Only rendered when annotationsEnabled is true.
 */

import { useAnnotations } from "../hooks/useAnnotations"
import type { InitialData } from "../types"
import { AnnotationOverlay } from "./AnnotationOverlay"
import { DevMenu } from "./DevMenu"

interface DevAnnotationLayerProps {
  initialData: InitialData
}

export function DevAnnotationLayer({ initialData }: DevAnnotationLayerProps) {
  const {
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
  } = useAnnotations()

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
        selectedAnnotation={selectedAnnotation}
        onCreate={create}
        onRemove={remove}
        onSelectAnnotation={selectAnnotation}
        onDeselectAnnotation={deselectAnnotation}
        onAcknowledge={acknowledge}
        onResolve={resolve}
        onDismiss={dismiss}
        onReply={reply}
      />
    </>
  )
}
