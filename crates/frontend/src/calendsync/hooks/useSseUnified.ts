/**
 * Unified SSE hook that works across web and Tauri.
 *
 * This hook provides a consistent interface for SSE across platforms:
 * - On web: Uses native EventSource via useWebSse
 * - On Tauri: Uses Tauri events via useTauriSse (routes through Rust backend)
 *
 * Both implementations update Dexie and provide the same callbacks.
 */

import { type UseWebSseConfig, type UseWebSseResult, useWebSse } from "./useWebSse"
import { useTauriSse } from "../../tauri/hooks/useTauriSse"

/** Detect if running in Tauri */
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window

// Re-export types for consumers
export type { UseWebSseConfig, UseWebSseResult }

/**
 * Hook for managing SSE that works on both web and Tauri.
 *
 * - Web: Uses native EventSource via useWebSse
 * - Tauri: Uses Tauri events via useTauriSse (Rust backend)
 *
 * @example
 * ```typescript
 * useSseUnified({
 *   calendarId: "abc123",
 *   enabled: true,
 *   onEntryAdded: (entry, date) => {
 *     console.log('Entry added:', entry)
 *   },
 * })
 * ```
 */
export function useSseUnified(config: UseWebSseConfig): UseWebSseResult {
  if (isTauri) {
    // biome-ignore lint/correctness/useHookAtTopLevel: isTauri is a module-level constant that never changes during component lifecycle
    return useTauriSse(config)
  }

  // biome-ignore lint/correctness/useHookAtTopLevel: isTauri is a module-level constant that never changes during component lifecycle
  return useWebSse(config)
}
