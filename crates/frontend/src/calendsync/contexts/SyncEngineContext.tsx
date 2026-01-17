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

  // Only create engine on client (requires IndexedDB)
  if (typeof window !== "undefined" && !engineRef.current) {
    engineRef.current = new SyncEngine(db)
  }

  // Initialize transport when available (client-side only)
  if (engineRef.current && transport && !engineRef.current.hasTransport()) {
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
 * @throws Error if used outside of SyncEngineProvider (on client)
 * @returns The SyncEngine instance, or null during SSR
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const engine = useSyncEngineContext()
 *   // Engine is null during SSR, check before use
 *   if (engine) {
 *     // Use engine for low-level access
 *   }
 * }
 * ```
 */
export function useSyncEngineContext(): SyncEngine | null {
  const engine = useContext(SyncEngineContext)
  // During SSR, engine will be null - that's expected
  if (typeof window !== "undefined" && !engine) {
    throw new Error("useSyncEngineContext must be used within a SyncEngineProvider")
  }
  return engine
}
