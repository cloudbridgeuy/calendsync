/**
 * React Context for SyncEngine.
 *
 * Provides a singleton SyncEngine instance to the component tree,
 * replacing the module-level singleton pattern for better testability
 * and React Strict Mode compatibility.
 */

import { useTransport } from "@core/transport"
import { createContext, type ReactNode, useContext, useEffect, useRef } from "react"

import { db } from "../db"
import { SyncEngine } from "../sync/engine"

/**
 * Context holding the SyncEngine instance.
 * Null when accessed outside of SyncEngineProvider.
 */
const SyncEngineContext = createContext<SyncEngine | null>(null)

/**
 * Props for SyncEngineProvider.
 */
export interface SyncEngineProviderProps {
  children: ReactNode
}

/**
 * Provider component that creates and manages the SyncEngine instance.
 *
 * The engine is created once on mount and persists for the lifetime
 * of the provider. It automatically initializes with the current
 * transport from TransportProvider.
 *
 * @example
 * ```tsx
 * function App() {
 *   return (
 *     <TransportProvider transport={transport}>
 *       <SyncEngineProvider>
 *         <Calendar />
 *       </SyncEngineProvider>
 *     </TransportProvider>
 *   )
 * }
 * ```
 */
export function SyncEngineProvider({ children }: SyncEngineProviderProps) {
  const transport = useTransport()
  const engineRef = useRef<SyncEngine | null>(null)

  // Create engine on first render
  if (!engineRef.current) {
    engineRef.current = new SyncEngine(db)
  }

  // Initialize transport when available
  // This is safe because transport from TransportProvider is stable
  if (transport && !engineRef.current.hasTransport()) {
    engineRef.current.initTransport(transport)
  }

  // Cleanup on unmount - dispose event listeners
  useEffect(() => {
    return () => {
      engineRef.current?.dispose()
    }
  }, [])

  return (
    <SyncEngineContext.Provider value={engineRef.current}>{children}</SyncEngineContext.Provider>
  )
}

/**
 * Hook to access the SyncEngine from context.
 *
 * Must be used within a SyncEngineProvider.
 *
 * @throws Error if used outside of SyncEngineProvider
 * @returns The SyncEngine instance
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const engine = useSyncEngineContext()
 *   // Use engine directly for low-level access
 * }
 * ```
 */
export function useSyncEngineContext(): SyncEngine {
  const engine = useContext(SyncEngineContext)
  if (!engine) {
    throw new Error("useSyncEngineContext must be used within a SyncEngineProvider")
  }
  return engine
}
