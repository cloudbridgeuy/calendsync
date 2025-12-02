/**
 * Client entry point for React hydration.
 * Hydrates the server-rendered HTML with event handlers.
 */

import { hydrateRoot } from "react-dom/client"

import { App } from "./App"
import { initControlPlaneUrl } from "./hooks/useApi"
import type { InitialData } from "./types"

// Read initial state from the server-embedded script
const initialData: InitialData = window.__INITIAL_DATA__

// Initialize API config with control plane URL from initial data
initControlPlaneUrl(initialData.controlPlaneUrl)

console.log("[Client] Hydrating React app...")
console.log(`[Client] Calendar ID: ${initialData.calendarId}`)
console.log(`[Client] Highlighted day: ${initialData.highlightedDay}`)

// Hydrate the React tree - attaches event handlers to existing DOM
hydrateRoot(document, <App initialData={initialData} />)

console.log("[Client] Hydration complete!")
