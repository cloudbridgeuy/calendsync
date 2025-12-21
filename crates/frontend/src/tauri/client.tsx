/**
 * Tauri CSR entry point for React rendering.
 * Unlike the web version which uses SSR + hydration, this uses createRoot for pure CSR.
 */

import { createRoot } from "react-dom/client"
import { App } from "./App"

// Configuration - hardcoded for now, will support deep links later
const API_URL = "http://localhost:3000"
// TODO: Replace with dynamic calendar selection or deep link parsing
const CALENDAR_ID = "placeholder-create-calendar-first"

console.log("[Tauri] Mounting React app...")
console.log(`[Tauri] API URL: ${API_URL}`)
console.log(`[Tauri] Calendar ID: ${CALENDAR_ID}`)

const root = document.getElementById("root")
if (!root) {
  throw new Error("Root element not found")
}

createRoot(root).render(<App apiUrl={API_URL} calendarId={CALENDAR_ID} />)
