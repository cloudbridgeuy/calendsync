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

        if (!sessionId) {
          if (mounted) setView("login")
          return
        }

        // Validate session with server
        const isValid = await transport.validateSession()
        if (!isValid) {
          await transport.clearSession()
          if (mounted) setView("login")
          return
        }

        if (mounted) setSession(sessionId)

        // Load calendar
        await loadCalendar()
      } catch (e) {
        if (mounted) {
          setError(`Failed to initialize: ${e}`)
          setView("login")
        }
      }
    }

    const loadCalendar = async () => {
      try {
        // Fetch user's calendars first
        const calendars = await transport.fetchMyCalendars()

        if (calendars.length === 0) {
          // User has no calendars - shouldn't happen normally
          if (mounted) setError("No calendars found for this account")
          return
        }

        // Try stored calendar first, but verify user has access
        const lastCalendarId = await transport.getLastCalendar()

        if (lastCalendarId) {
          // Check if user has access to this calendar
          const hasAccess = calendars.some((cal) => cal.id === lastCalendarId)

          if (hasAccess) {
            // User has access - use stored calendar
            if (mounted) {
              setCalendar(lastCalendarId)
              setView("calendar")
            }
            return
          }

          // User doesn't have access - clear stale ID
          await transport.clearLastCalendar()
        }

        // Use first calendar (usually the user's default/main calendar)
        const calendarId = calendars[0].id
        await transport.setLastCalendar(calendarId)
        if (mounted) {
          setCalendar(calendarId)
          setView("calendar")
        }
      } catch (e: unknown) {
        const error = e as Error
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

    init()

    return () => {
      mounted = false
      unlistenCodePromise.then((fn) => fn())
      unlistenErrorPromise.then((fn) => fn())
    }
  }, [transport, setView, setSession, setCalendar, setError])
}
