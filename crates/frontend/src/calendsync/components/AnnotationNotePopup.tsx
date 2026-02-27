/**
 * Popup for entering annotation notes when clicking on an element.
 */

import { useEffect, useRef, useState } from "react"
import type { AnnotationIntent, AnnotationSeverity } from "../../core/calendar/annotations"

interface AnnotationNotePopupProps {
  position: { top: number; left: number }
  onSave: (comment: string, intent: AnnotationIntent, severity: AnnotationSeverity) => void
  onCancel: () => void
}

export function AnnotationNotePopup({ position, onSave, onCancel }: AnnotationNotePopupProps) {
  const [comment, setComment] = useState("")
  const [intent, setIntent] = useState<AnnotationIntent>("fix")
  const [severity, setSeverity] = useState<AnnotationSeverity>("suggestion")
  const inputRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    inputRef.current?.focus()
  }, [])

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        onCancel()
      } else if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
        if (comment.trim()) onSave(comment.trim(), intent, severity)
      }
    }
    document.addEventListener("keydown", handleKeyDown)
    return () => document.removeEventListener("keydown", handleKeyDown)
  }, [comment, intent, severity, onSave, onCancel])

  return (
    <div
      className="annotation-note-popup"
      style={{
        position: "fixed",
        top: Math.min(position.top, window.innerHeight - 200),
        left: Math.min(position.left, window.innerWidth - 320),
      }}
    >
      <div className="annotation-note-header">Add Annotation</div>
      <textarea
        ref={inputRef}
        className="annotation-note-input"
        placeholder="Describe the issue..."
        value={comment}
        onChange={(e) => setComment(e.target.value)}
        rows={3}
      />
      <div className="annotation-note-selects">
        <select
          className="annotation-note-select"
          value={intent}
          onChange={(e) => setIntent(e.target.value as AnnotationIntent)}
        >
          <option value="fix">Fix</option>
          <option value="change">Change</option>
          <option value="question">Question</option>
          <option value="approve">Approve</option>
        </select>
        <select
          className="annotation-note-select"
          value={severity}
          onChange={(e) => setSeverity(e.target.value as AnnotationSeverity)}
        >
          <option value="suggestion">Suggestion</option>
          <option value="important">Important</option>
          <option value="blocking">Blocking</option>
        </select>
      </div>
      <div className="annotation-note-actions">
        <button type="button" className="annotation-note-cancel" onClick={onCancel}>
          Cancel
        </button>
        <button
          type="button"
          className="annotation-note-save"
          onClick={() => comment.trim() && onSave(comment.trim(), intent, severity)}
          disabled={!comment.trim()}
        >
          Save
        </button>
      </div>
      <div className="annotation-note-hint">Ctrl+Enter to save, Esc to cancel</div>
    </div>
  )
}
