/**
 * Client entry point for React hydration.
 * Hydrates the server-rendered HTML with event handlers.
 */

import { createWebTransport, TransportProvider } from "@core/transport"
import { hydrateRoot } from "react-dom/client"
import { App } from "./App"
import { SyncEngineProvider } from "./contexts"
import { initControlPlaneUrl } from "./hooks/useApi"
import type { InitialData } from "./types"

// Read initial state from the server-embedded script
const initialData: InitialData = window.__INITIAL_DATA__

// Initialize API config with control plane URL from initial data
// (kept for SSE URL compatibility - will be removed when SSE uses transport)
initControlPlaneUrl(initialData.controlPlaneUrl)

// Create web transport for HTTP operations
const transport = createWebTransport(initialData.controlPlaneUrl)

console.log("[Client] Hydrating React app...")
console.log(`[Client] Calendar ID: ${initialData.calendarId}`)
console.log(`[Client] Highlighted day: ${initialData.highlightedDay}`)

// Hydrate the React tree - attaches event handlers to existing DOM
hydrateRoot(
  document,
  <TransportProvider transport={transport}>
    <SyncEngineProvider>
      <App initialData={initialData} />
    </SyncEngineProvider>
  </TransportProvider>,
)

console.log("[Client] Hydration complete!")
