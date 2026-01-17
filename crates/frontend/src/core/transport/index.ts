/**
 * Transport layer exports.
 *
 * Re-exports types, context, and implementations for convenient imports.
 */

export { TransportProvider, useTransport } from "./context"
export { createSsrTransport } from "./ssr"
export { createTauriTransport } from "./tauri"
export type {
  CalendarWithRole,
  CreateEntryPayload,
  FetchEntriesOptions,
  Transport,
} from "./types"
export { createWebTransport } from "./web"
