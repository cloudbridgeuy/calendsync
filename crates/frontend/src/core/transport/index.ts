/**
 * Transport layer exports.
 *
 * Re-exports types, context, and implementations for convenient imports.
 */

export type {
  CalendarWithRole,
  CreateEntryPayload,
  FetchEntriesOptions,
  Transport,
} from "./types"

export { TransportProvider, useTransport } from "./context"
export { createTauriTransport } from "./tauri"
