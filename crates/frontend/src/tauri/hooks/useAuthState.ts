import { useCallback, useState } from "react"
import type { AppView, AuthState } from "../types"

export function useAuthState() {
  const [state, setState] = useState<AuthState>({
    view: "loading",
    sessionId: null,
    calendarId: null,
    error: null,
  })

  const setView = useCallback((view: AppView) => {
    setState((s) => ({ ...s, view }))
  }, [])

  const setSession = useCallback((sessionId: string | null) => {
    setState((s) => ({ ...s, sessionId }))
  }, [])

  const setCalendar = useCallback((calendarId: string | null) => {
    setState((s) => ({ ...s, calendarId }))
  }, [])

  const setError = useCallback((error: string | null) => {
    setState((s) => ({ ...s, error }))
  }, [])

  return { state, setView, setSession, setCalendar, setError }
}
