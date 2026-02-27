/**
 * Detail popup shown when an annotation marker is clicked.
 * Displays annotation metadata, thread messages, and status transition actions.
 */

import { useEffect, useRef, useState } from "react"
import {
  type Annotation,
  type AnnotationStatus,
  intentIcon,
  severityLabel,
  statusColor,
  truncateTextContent,
} from "../../core/calendar/annotations"

interface AnnotationDetailPopupProps {
  annotation: Annotation
  onAcknowledge: (id: string) => void
  onResolve: (id: string, summary: string) => void
  onDismiss: (id: string, reason: string) => void
  onReply: (id: string, message: string) => void
  onDelete: (id: string) => void
  onClose: () => void
}

type InlineForm = "resolve" | "dismiss" | null

function isTerminalStatus(status: AnnotationStatus): boolean {
  return status === "resolved" || status === "dismissed"
}

export function AnnotationDetailPopup({
  annotation,
  onAcknowledge,
  onResolve,
  onDismiss,
  onReply,
  onDelete,
  onClose,
}: AnnotationDetailPopupProps) {
  const [replyText, setReplyText] = useState("")
  const [resolveSummary, setResolveSummary] = useState("")
  const [dismissReason, setDismissReason] = useState("")
  const [activeForm, setActiveForm] = useState<InlineForm>(null)
  const [confirmDelete, setConfirmDelete] = useState(false)
  const replyRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    replyRef.current?.focus()
  }, [])

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        if (activeForm) {
          setActiveForm(null)
        } else if (confirmDelete) {
          setConfirmDelete(false)
        } else {
          onClose()
        }
      }
    }
    document.addEventListener("keydown", handleKeyDown)
    return () => document.removeEventListener("keydown", handleKeyDown)
  }, [activeForm, confirmDelete, onClose])

  function handleReplySubmit() {
    const trimmed = replyText.trim()
    if (!trimmed) return
    onReply(annotation.id, trimmed)
    setReplyText("")
  }

  function handleResolveSubmit() {
    const trimmed = resolveSummary.trim()
    if (!trimmed) return
    onResolve(annotation.id, trimmed)
    setActiveForm(null)
    setResolveSummary("")
  }

  function handleDismissSubmit() {
    const trimmed = dismissReason.trim()
    if (!trimmed) return
    onDismiss(annotation.id, trimmed)
    setActiveForm(null)
    setDismissReason("")
  }

  function handleDeleteClick() {
    if (confirmDelete) {
      onDelete(annotation.id)
    } else {
      setConfirmDelete(true)
    }
  }

  const componentLabel = annotation.component_name
    ? `${annotation.element_path} (${annotation.component_name})`
    : annotation.element_path

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: Backdrop click-to-close is intentional
    <div className="annotation-detail-backdrop" onClick={onClose} role="presentation">
      <div
        className="annotation-detail-popup"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label="Annotation details"
      >
        {/* Header */}
        <div className="annotation-detail-header">
          <span className="annotation-detail-path">{componentLabel}</span>
          <span
            className="annotation-detail-status-badge"
            style={{ backgroundColor: statusColor(annotation.status) }}
          >
            {annotation.status}
          </span>
        </div>

        {/* Meta section */}
        <div className="annotation-detail-meta">
          <p className="annotation-detail-comment">{annotation.comment}</p>
          <div className="annotation-detail-meta-row">
            <span>
              {intentIcon(annotation.intent)} {annotation.intent}
            </span>
            <span>{severityLabel(annotation.severity)}</span>
          </div>
          <div className="annotation-detail-meta-row">
            <span>&lt;{annotation.tag_name}&gt;</span>
            {annotation.text_content && (
              <span className="annotation-detail-text-preview">
                {truncateTextContent(annotation.text_content, 80)}
              </span>
            )}
          </div>
        </div>

        {/* Thread section */}
        <div className="annotation-detail-thread">
          {annotation.thread.length > 0 && (
            <ul className="annotation-detail-thread-list">
              {annotation.thread.map((msg) => (
                <li key={msg.id} className="annotation-detail-thread-message">
                  <span className="annotation-detail-thread-author">{msg.author}</span>
                  <span className="annotation-detail-thread-text">{msg.message}</span>
                </li>
              ))}
            </ul>
          )}

          <div className="annotation-detail-reply">
            <textarea
              ref={replyRef}
              className="annotation-detail-reply-input"
              placeholder="Write a reply..."
              value={replyText}
              onChange={(e) => setReplyText(e.target.value)}
              rows={2}
            />
            <button
              type="button"
              className="annotation-detail-reply-send"
              onClick={handleReplySubmit}
              disabled={!replyText.trim()}
            >
              Send
            </button>
          </div>
        </div>

        {/* Inline forms for resolve / dismiss */}
        {activeForm === "resolve" && (
          <div className="annotation-detail-inline-form">
            <label htmlFor="resolve-summary">Resolution summary</label>
            <textarea
              id="resolve-summary"
              value={resolveSummary}
              onChange={(e) => setResolveSummary(e.target.value)}
              placeholder="Describe what was done..."
              rows={2}
            />
            <div className="annotation-detail-inline-form-actions">
              <button type="button" onClick={() => setActiveForm(null)}>
                Cancel
              </button>
              <button type="button" onClick={handleResolveSubmit} disabled={!resolveSummary.trim()}>
                Confirm Resolve
              </button>
            </div>
          </div>
        )}

        {activeForm === "dismiss" && (
          <div className="annotation-detail-inline-form">
            <label htmlFor="dismiss-reason">Dismiss reason</label>
            <textarea
              id="dismiss-reason"
              value={dismissReason}
              onChange={(e) => setDismissReason(e.target.value)}
              placeholder="Why is this being dismissed..."
              rows={2}
            />
            <div className="annotation-detail-inline-form-actions">
              <button type="button" onClick={() => setActiveForm(null)}>
                Cancel
              </button>
              <button type="button" onClick={handleDismissSubmit} disabled={!dismissReason.trim()}>
                Confirm Dismiss
              </button>
            </div>
          </div>
        )}

        {/* Actions footer */}
        <div className="annotation-detail-actions">
          {annotation.status === "pending" && (
            <button
              type="button"
              className="annotation-detail-action-btn"
              onClick={() => onAcknowledge(annotation.id)}
            >
              Acknowledge
            </button>
          )}

          {!isTerminalStatus(annotation.status) && (
            <>
              <button
                type="button"
                className="annotation-detail-action-btn"
                onClick={() => setActiveForm("resolve")}
              >
                Resolve
              </button>
              <button
                type="button"
                className="annotation-detail-action-btn"
                onClick={() => setActiveForm("dismiss")}
              >
                Dismiss
              </button>
            </>
          )}

          <button
            type="button"
            className="annotation-detail-action-btn annotation-detail-action-delete"
            onClick={handleDeleteClick}
          >
            {confirmDelete ? "Confirm Delete" : "Delete"}
          </button>

          <button
            type="button"
            className="annotation-detail-action-btn annotation-detail-action-close"
            onClick={onClose}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  )
}
