/**
 * Transport context for React components.
 *
 * Provides the Transport instance to the component tree via React Context,
 * allowing hooks and components to access HTTP operations without knowing
 * the underlying implementation (web fetch vs Tauri invoke).
 */

import { createContext, useContext, type ReactNode } from "react"
import type { Transport } from "./types"

const TransportContext = createContext<Transport | null>(null)

interface TransportProviderProps {
  transport: Transport
  children: ReactNode
}

/**
 * Provider component that makes a Transport instance available to descendants.
 */
export function TransportProvider({ transport, children }: TransportProviderProps) {
  return <TransportContext.Provider value={transport}>{children}</TransportContext.Provider>
}

/**
 * Hook to access the Transport instance from context.
 *
 * @throws Error if used outside of TransportProvider
 */
export function useTransport(): Transport {
  const transport = useContext(TransportContext)
  if (!transport) {
    throw new Error("useTransport must be used within TransportProvider")
  }
  return transport
}
