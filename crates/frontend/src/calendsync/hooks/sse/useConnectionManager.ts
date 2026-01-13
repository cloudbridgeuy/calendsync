/**
 * Shared connection state management for SSE hooks.
 *
 * Extracts the common pattern of:
 * - Managing connection state
 * - Storing callbacks in refs to avoid unnecessary reconnections
 * - Providing an updateConnectionState function that updates both state and callback
 *
 * Used by: useWebSse, useTauriSse
 */

import type { SseConnectionState } from "@core/sse/types"
import { useCallback, useEffect, useRef, useState } from "react"

/**
 * Configuration for the connection manager hook.
 */
export interface ConnectionConfig {
  /** Callback when connection state changes */
  onConnectionChange?: (state: SseConnectionState) => void
}

/**
 * Result from the connection manager hook.
 */
export interface ConnectionManager {
  /** Current connection state */
  connectionState: SseConnectionState
  /** Update connection state and notify callback */
  updateConnectionState: (state: SseConnectionState) => void
}

/**
 * Hook for managing SSE connection state.
 *
 * This hook extracts the common connection state management pattern
 * shared across useWebSse and useTauriSse.
 *
 * @param config - Connection configuration
 * @returns Connection manager with state and update function
 *
 * @example
 * ```typescript
 * function useMySSE(config: { onConnectionChange?: (state) => void }) {
 *   const { connectionState, updateConnectionState } = useConnectionManager({
 *     onConnectionChange: config.onConnectionChange,
 *   })
 *
 *   // Use updateConnectionState when connection opens/closes/errors
 *   eventSource.onopen = () => updateConnectionState("connected")
 *
 *   return { connectionState }
 * }
 * ```
 */
export function useConnectionManager(config: ConnectionConfig): ConnectionManager {
  const { onConnectionChange } = config

  const [connectionState, setConnectionState] = useState<SseConnectionState>("disconnected")

  // Store callbacks in refs to avoid reconnecting on callback changes
  const callbacksRef = useRef<Pick<ConnectionConfig, "onConnectionChange">>({
    onConnectionChange,
  })

  // Update callback refs when callbacks change
  useEffect(() => {
    callbacksRef.current = {
      onConnectionChange,
    }
  }, [onConnectionChange])

  /**
   * Update connection state and notify callback.
   * This is memoized to avoid unnecessary re-renders.
   */
  const updateConnectionState = useCallback((state: SseConnectionState) => {
    setConnectionState(state)
    callbacksRef.current.onConnectionChange?.(state)
  }, [])

  return {
    connectionState,
    updateConnectionState,
  }
}
