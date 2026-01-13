/**
 * API configuration for SSE and fallback fetch operations.
 *
 * Note: Most HTTP operations should use the Transport layer instead.
 * This module provides:
 * - Control plane URL initialization for SSE connections
 * - Legacy support for SyncEngine's fallback API client
 */

// CONTROL_PLANE_URL is set from __INITIAL_DATA__ at hydration time
let CONTROL_PLANE_URL = ""

/**
 * Initialize the control plane URL from initial data.
 * This should be called once during hydration.
 */
export function initControlPlaneUrl(url: string): void {
  CONTROL_PLANE_URL = url
}

/**
 * Get the control plane URL.
 * Used by SSE hooks and SyncEngine's fallback API client.
 */
export function getControlPlaneUrl(): string {
  return CONTROL_PLANE_URL
}
