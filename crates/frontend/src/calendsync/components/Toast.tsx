/**
 * Toast notification component for SSE events.
 */

import { useEffect, useState } from "react"

export type ToastType = "added" | "updated" | "deleted"

export interface ToastData {
    id: string
    type: ToastType
    title: string
    date: string
}

interface ToastProps {
    toast: ToastData
    onRemove: (id: string) => void
}

const TOAST_DURATION = 3000

function Toast({ toast, onRemove }: ToastProps) {
    const [isExiting, setIsExiting] = useState(false)

    useEffect(() => {
        const timer = setTimeout(() => {
            setIsExiting(true)
            setTimeout(() => onRemove(toast.id), 300)
        }, TOAST_DURATION)

        return () => clearTimeout(timer)
    }, [toast.id, onRemove])

    const icons: Record<ToastType, string> = {
        added: "+",
        updated: "~",
        deleted: "-",
    }

    const labels: Record<ToastType, string> = {
        added: "Added",
        updated: "Updated",
        deleted: "Deleted",
    }

    return (
        <div className={`toast toast-${toast.type}${isExiting ? " exiting" : ""}`}>
            <span className="toast-icon">{icons[toast.type]}</span>
            <div className="toast-message">
                <span>{labels[toast.type]}: </span>
                <span className="toast-title">{toast.title}</span>
            </div>
        </div>
    )
}

interface ToastContainerProps {
    toasts: ToastData[]
    onRemove: (id: string) => void
}

export function ToastContainer({ toasts, onRemove }: ToastContainerProps) {
    if (toasts.length === 0) return null

    return (
        <div className="toast-container">
            {toasts.map((toast) => (
                <Toast key={toast.id} toast={toast} onRemove={onRemove} />
            ))}
        </div>
    )
}
