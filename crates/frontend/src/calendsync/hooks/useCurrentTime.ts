import { useEffect, useState } from "react"

/**
 * Returns a Date that refreshes every `intervalMs` milliseconds.
 * Used to drive the now-indicator position in the schedule view.
 */
export function useCurrentTime(intervalMs = 30_000): Date {
  const [now, setNow] = useState(() => new Date())

  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), intervalMs)
    return () => clearInterval(id)
  }, [intervalMs])

  return now
}
