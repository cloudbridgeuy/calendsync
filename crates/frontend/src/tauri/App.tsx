import { Calendar } from "@calendsync/components"
import type { InitialData } from "@calendsync/types"
import { formatDateKey } from "@core/calendar/dates"
import { useTransport } from "@core/transport"
import { useEffect, useState } from "react"
import { TauriLoginPage } from "./components"
import { useAppInit, useAuthState } from "./hooks"

export function App() {
  const transport = useTransport()
  const { state, setView, setSession, setCalendar, setError } = useAuthState()
  const [initialData, setInitialData] = useState<InitialData | null>(null)

  useAppInit({ setView, setSession, setCalendar, setError })

  // Fetch calendar data when calendarId is set
  useEffect(() => {
    if (!state.calendarId || !state.sessionId) return

    // Capture non-null values for use in async function
    const calendarId = state.calendarId

    const fetchCalendarData = async () => {
      const highlightedDay = formatDateKey(new Date())

      try {
        const days = await transport.fetchEntries({
          calendarId,
          highlightedDay,
        })

        setInitialData({
          calendarId,
          highlightedDay,
          days,
          clientBundleUrl: "",
          controlPlaneUrl: "", // Not needed for Tauri - transport handles it
          sseEnabled: false, // Disabled in Tauri - EventSource bypasses transport layer
        })
      } catch (e) {
        const message = e instanceof Error ? e.message : String(e)
        if (message.includes("401") || message === "UNAUTHORIZED") {
          await transport.clearSession()
          setSession(null)
          setView("login")
          return
        }
        setError(`Failed to load calendar: ${message}`)
      }
    }

    fetchCalendarData()
  }, [state.calendarId, state.sessionId, transport, setSession, setView, setError])

  // Render based on view state
  if (state.view === "loading") {
    return (
      <div className="tauri-loading">
        <p>Loading...</p>
      </div>
    )
  }

  if (state.view === "login") {
    return <TauriLoginPage onError={setError} />
  }

  if (state.error) {
    return (
      <div className="tauri-error">
        <h1>Error</h1>
        <p>{state.error}</p>
        <button type="button" onClick={() => setError(null)}>
          Dismiss
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
