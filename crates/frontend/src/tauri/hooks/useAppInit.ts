import { determineInitialView, selectCalendar } from "@core/calendar/selection"
import { useTransport } from "@core/transport"
import { listen } from "@tauri-apps/api/event"
import { useEffect } from "react"
import type { AppView } from "../types"

interface UseAppInitOptions {
  setView: (view: AppView) => void
  setSession: (sessionId: string | null) => void
  setCalendar: (calendarId: string | null) => void
  setError: (error: string | null) => void
}

export function useAppInit(options: UseAppInitOptions) {
  const { setView, setSession, setCalendar, setError } = options
  const transport = useTransport()

  useEffect(() => {
    let mounted = true

    const init = async () => {
      try {
        // Check for stored session
        const sessionId = await transport.getSession()

        // Validate session with server (only if we have a session)
        const isValid = sessionId ? await transport.validateSession() : false

        // Determine initial view using pure function
        const view = determineInitialView(sessionId, isValid)

        if (view === "login") {
          if (sessionId) {
            // Had a session but it was invalid - clear it
            await transport.clearSession()
          }
          if (mounted) setView("login")
          return
        }

        // view === "loading_calendar" - valid session
        if (mounted) setSession(sessionId)

        // Load calendar
        await loadCalendar()
      } catch (e) {
        if (mounted) {
          setError(`Failed to initialize: ${e instanceof Error ? e.message : String(e)}`)
          setView("login")
        }
      }
    }

    const loadCalendar = async () => {
      try {
        // Fetch user's calendars first
        const calendars = await transport.fetchMyCalendars()

        // Get stored calendar preference
        const lastCalendarId = await transport.getLastCalendar()

        // Use pure function to determine calendar selection
        const selection = selectCalendar(lastCalendarId, calendars)

        if (selection.type === "no_calendars") {
          // User has no calendars - shouldn't happen normally
          if (mounted) setError("No calendars found for this account")
          return
        }

        if (selection.type === "use_stored") {
          // Using stored calendar - already saved
          if (mounted) {
            setCalendar(selection.calendarId)
            setView("calendar")
          }
          return
        }

        // selection.type === "use_default"
        // Clear stale stored ID if we had one that wasn't valid
        if (lastCalendarId) {
          await transport.clearLastCalendar()
        }

        // Save and use the default calendar
        await transport.setLastCalendar(selection.calendarId)
        if (mounted) {
          setCalendar(selection.calendarId)
          setView("calendar")
        }
      } catch (e: unknown) {
        const error = e instanceof Error ? e : new Error(String(e))
        if (error.message === "UNAUTHORIZED") {
          await transport.clearSession()
          if (mounted) {
            setSession(null)
            setView("login")
          }
        } else {
          if (mounted) setError(`Failed to load calendar: ${error.message}`)
        }
      }
    }

    // Listen for auth code events from deep link handler
    // Rust backend exchanges the code via HTTP
    const unlistenCodePromise = listen<{ code: string; state: string }>(
      "auth-code-received",
      async (event) => {
        try {
          const { code, state } = event.payload
          // Exchange code for session via Rust backend
          const sessionId = await transport.exchangeAuthCode(code, state)
          // Session is already saved by the Rust backend
          if (mounted) {
            setSession(sessionId)
            await loadCalendar()
          }
        } catch (e) {
          if (mounted) {
            const error = e instanceof Error ? e.message : String(e)
            setError(`Authentication failed: ${error}`)
          }
        }
      },
    )

    // Listen for auth error events
    const unlistenErrorPromise = listen<{ error: string }>("auth-error", (event) => {
      if (mounted) setError(`Authentication failed: ${event.payload.error}`)
    })

    // Listen for dev session from environment variable
    // This is used to transfer a session from web app to desktop app in dev mode
    const unlistenDevSessionPromise = listen<string>("dev-session-detected", async (event) => {
      if (!mounted) return
      const sessionId = event.payload
      console.log("[Dev] Dev session detected from environment")

      try {
        // Save the session
        await transport.setSession(sessionId)
        setSession(sessionId)

        // Validate it works
        const isValid = await transport.validateSession()
        if (!isValid) {
          console.warn("[Dev] Dev session is invalid or expired")
          setError("Dev session is invalid or expired. Please log in again via web.")
          await transport.clearSession()
          setSession(null)
          setView("login")
          return
        }

        // Load the calendar
        await loadCalendar()
      } catch (e) {
        console.error("[Dev] Failed to use dev session:", e)
        setError(`Failed to use dev session: ${e instanceof Error ? e.message : String(e)}`)
      }
    })

    init()

    return () => {
      mounted = false
      unlistenCodePromise.then((fn) => fn())
      unlistenErrorPromise.then((fn) => fn())
      unlistenDevSessionPromise.then((fn) => fn())
    }
  }, [transport, setView, setSession, setCalendar, setError])
}
