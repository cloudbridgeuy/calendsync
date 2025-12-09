/**
 * Tauri App wrapper - fetches initial data and renders Calendar.
 * Unlike web App.tsx, this does NOT render full HTML document.
 * It fetches data on mount since there's no SSR to embed it.
 */

import { Calendar } from "@calendsync/components"
import { initControlPlaneUrl } from "@calendsync/hooks/useApi"
import type { InitialData } from "@calendsync/types"
import { formatDateKey } from "@core/calendar/dates"
import type { ServerDay } from "@core/calendar/types"
import { useEffect, useState } from "react"

interface AppProps {
  apiUrl: string
  calendarId: string
}

/**
 * Tauri App component.
 * Fetches initial calendar data on mount and renders the Calendar.
 */
export function App({ apiUrl, calendarId }: AppProps) {
  const [initialData, setInitialData] = useState<InitialData | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    // Initialize API URL for all hooks
    initControlPlaneUrl(apiUrl)

    // Fetch initial data
    const highlightedDay = formatDateKey(new Date())
    const url = `${apiUrl}/api/entries/calendar?calendar_id=${calendarId}&highlighted_day=${highlightedDay}&before=3&after=3`

    console.log(`[Tauri] Fetching initial data from: ${url}`)

    fetch(url)
      .then((res) => {
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}: ${res.statusText}`)
        }
        return res.json() as Promise<ServerDay[]>
      })
      .then((days) => {
        console.log(`[Tauri] Received ${days.length} days of data`)
        setInitialData({
          calendarId,
          highlightedDay,
          days,
          clientBundleUrl: "", // Not needed for CSR
          controlPlaneUrl: apiUrl,
        })
      })
      .catch((err) => {
        console.error("[Tauri] Failed to fetch initial data:", err)
        setError(err.message)
      })
  }, [apiUrl, calendarId])

  if (error) {
    return (
      <div className="tauri-error">
        <h1>Failed to load calendar</h1>
        <p>{error}</p>
        <p>Make sure the calendsync server is running at {apiUrl}</p>
        <button type="button" onClick={() => window.location.reload()}>
          Retry
        </button>
      </div>
    )
  }

  if (!initialData) {
    return (
      <div className="tauri-loading">
        <p>Loading calendar...</p>
      </div>
    )
  }

  return (
    <Calendar initialData={initialData}>
      <Calendar.Header />
      <Calendar.NotificationCenter />
      <Calendar.Days />
      <Calendar.TodayButton />
      <Calendar.Fab />
      <Calendar.Modal />
    </Calendar>
  )
}
