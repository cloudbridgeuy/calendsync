/**
 * Popup for entering annotation notes when clicking on an element.
 */

import { useEffect, useRef, useState } from "react"

interface AnnotationNotePopupProps {
  position: { top: number; left: number }
  onSave: (note: string) => void
  onCancel: () => void
}

export function AnnotationNotePopup({ position, onSave, onCancel }: AnnotationNotePopupProps) {
  const [note, setNote] = useState("")
  const inputRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    inputRef.current?.focus()
  }, [])

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        onCancel()
      } else if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
        if (note.trim()) onSave(note.trim())
      }
    }
    document.addEventListener("keydown", handleKeyDown)
    return () => document.removeEventListener("keydown", handleKeyDown)
  }, [note, onSave, onCancel])

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
        value={note}
        onChange={(e) => setNote(e.target.value)}
        rows={3}
      />
      <div className="annotation-note-actions">
        <button type="button" className="annotation-note-cancel" onClick={onCancel}>
          Cancel
        </button>
        <button
          type="button"
          className="annotation-note-save"
          onClick={() => note.trim() && onSave(note.trim())}
          disabled={!note.trim()}
        >
          Save
        </button>
      </div>
      <div className="annotation-note-hint">Ctrl+Enter to save, Esc to cancel</div>
    </div>
  )
}
