/**
 * Shared SSE hooks for connection management and Dexie integration.
 *
 * These hooks extract common patterns used across:
 * - useWebSse (web browser SSE with Dexie persistence)
 * - useTauriSse (Tauri desktop/mobile SSE via Rust backend)
 * - useSseUnified (platform-agnostic SSE selector)
 */

export type {
  ConnectionCallbacks,
  ConnectionConfig,
  ConnectionManager,
} from "./useConnectionManager"
export { useConnectionManager } from "./useConnectionManager"
export type { DexieHandlers } from "./useDexieHandlers"
export { useDexieHandlers } from "./useDexieHandlers"
